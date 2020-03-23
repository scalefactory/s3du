// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use crate::common::{
    ClientConfig,
    S3ObjectVersions,
};
use log::debug;
use rusoto_s3::{
    ListObjectsV2Request,
    ListObjectVersionsRequest,
    S3,
    S3Client,
};
use super::bucket_list::BucketList;

pub struct Client {
    pub client:          S3Client,
    pub buckets:         Option<BucketList>,
    pub object_versions: S3ObjectVersions,
}

impl Client {
    // Return a new CloudWatchClient in the specified region.
    pub fn new(config: ClientConfig) -> Self {
        let region = config.region;

        debug!(
            "new: Creating S3Client in region '{}'",
            region.name(),
        );

        let client = S3Client::new(region);

        Client {
            client:          client,
            buckets:         None,
            object_versions: config.s3_object_versions,
        }
    }

    // List object versions and filter according to S3ObjectVersions
    async fn size_object_versions(&self, bucket: &str) -> Result<usize> {
        let mut next_key_marker        = None;
        let mut next_version_id_marker = None;
        let mut size                   = 0;

        loop {
            let input = ListObjectVersionsRequest {
                bucket:            bucket.into(),
                key_marker:        next_key_marker.to_owned(),
                version_id_marker: next_version_id_marker.to_owned(),
                ..Default::default()
            };

            let output = self.client.list_object_versions(input).await?;

            // Depending on which object versions we're paying attention to,
            // we may or may not filter here.
            if let Some(versions) = output.versions {
                size += versions
                    .iter()
                    .filter_map(|v| {
                        // Here we take out object version selection into
                        // account. We only return v.size if we care about that
                        // object version.
                        let is_latest = v.is_latest.unwrap();

                        match self.object_versions {
                            S3ObjectVersions::All     => v.size,
                            S3ObjectVersions::Current => {
                                match is_latest {
                                    true  => v.size,
                                    false => None,
                                }
                            },
                            S3ObjectVersions::NonCurrent => {
                                match is_latest {
                                    true  => None,
                                    false => v.size,
                                }
                            },
                        }
                    })
                    .sum::<i64>() as usize;
            }

            match output.is_truncated {
                Some(true) => {
                    let nkm  = output.next_key_marker;
                    let nvim = output.next_version_id_marker;

                    next_key_marker        = nkm;
                    next_version_id_marker = nvim;
                },
                _ => break,
            }
        }

        Ok(size)
    }

    // Return the size of current object versions in the bucket. Handles paging
    // so should work on large buckets.
    async fn size_current_objects(&self, bucket: &str) -> Result<usize> {
        let mut continuation_token = None;
        let mut size               = 0;

        // Loop until all objects are processed.
        loop {
            let input = ListObjectsV2Request {
                bucket:             bucket.into(),
                continuation_token: continuation_token.to_owned(),
                ..Default::default()
            };

            let output = self.client.list_objects_v2(input).await?;

            // Process the contents and add up the sizes
            if let Some(contents) = output.contents {
                size += contents
                    .iter()
                    .filter_map(|o| o.size)
                    .sum::<i64>() as usize;
            }

            // If the output was truncated (Some(true)), we should have a
            // next_continuation_token.
            // If it wasn't, (Some(false) | None) we're done and can break.
            match output.is_truncated {
                Some(true) => {
                    let nct = output.next_continuation_token;
                    continuation_token = nct;
                },
                _ => break,
            }
        }

        Ok(size)
    }

    // A wrapper to call the appropriate bucket listing functions
    pub async fn size_objects(&self, bucket: &str) -> Result<usize> {
        match self.object_versions {
            S3ObjectVersions::All => {
                self.size_object_versions(bucket).await
            },
            S3ObjectVersions::Current => {
                self.size_current_objects(bucket).await
            },
            S3ObjectVersions::NonCurrent => {
                self.size_object_versions(bucket).await
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
        ReadMockResponse,
    };
    use tokio::runtime::Runtime;

    // Create a mock S3 client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Option<&str>,
        versions:  S3ObjectVersions,
    ) -> Client {
        let data = match data_file {
            None    => "".to_string(),
            Some(d) => MockResponseReader::read_response("test-data", d.into()),
        };

        let client = S3Client::new_with(
            MockRequestDispatcher::default().with_body(&data),
            MockCredentialsProvider,
            Default::default()
        );

        Client {
            client:          client,
            buckets:         None,
            object_versions: versions,
        }
    }

    #[test]
    fn test_size_objects_current() {
        let mut client = mock_client(
            Some("s3-list-objects.xml"),
            S3ObjectVersions::Current,
        );

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::size_objects(&mut client, "test-bucket"))
            .unwrap();

        let expected = 33_792;

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_size_objects_all_noncurrent() {
        // Current expects 0 here because our mock client won't return any
        // objects. This is handled in another test.
        let tests = vec![
            (S3ObjectVersions::All,        600_732),
            (S3ObjectVersions::Current,    0),
            (S3ObjectVersions::NonCurrent, 166_498),
        ];

        for test in tests {
            let versions      = test.0;
            let expected_size = test.1;

            let mut client = mock_client(
                Some("s3-list-object-versions.xml"),
                versions,
            );

            let ret = Runtime::new()
                .unwrap()
                .block_on(Client::size_objects(&mut client, "test-bucket"))
                .unwrap();

            assert_eq!(ret, expected_size);
        }
    }
}
