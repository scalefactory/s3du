// Implements the S3 Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use crate::common::{
    BucketNames,
    ClientConfig,
    S3ObjectVersions,
};
use log::debug;
use rusoto_core::Region;
use rusoto_s3::{
    GetBucketLocationRequest,
    ListObjectsV2Request,
    ListObjectVersionsRequest,
    S3,
    S3Client,
};
use std::str::FromStr;

/// The S3 `Client`.
pub struct Client {
    /// The Rusoto `S3Client`.
    pub client: S3Client,

    /// Selected bucket name, if any.
    pub bucket_name: Option<String>,

    /// Configuration for which objects to list in the bucket.
    pub object_versions: S3ObjectVersions,

    /// `Region` that we're listing buckets in.
    pub region: Region,
}

impl Client {
    /// Return a new S3 `Client` with the given `ClientConfig`.
    pub fn new(config: ClientConfig) -> Self {
        let bucket_name = config.bucket_name;
        let region      = config.region;

        debug!(
            "new: Creating S3Client in region '{}'",
            region.name(),
        );

        let client = S3Client::new(region.to_owned());

        Client {
            client:          client,
            bucket_name:     bucket_name,
            object_versions: config.s3_object_versions,
            region:          region,
        }
    }

    /// Returns a list of bucket names.
    pub async fn list_buckets(&self) -> Result<BucketNames> {
        let output = self.client.list_buckets().await?;

        let bucket_names = match output.buckets {
            Some(buckets) => {
                buckets.iter()
                    .filter_map(|b| b.name.to_owned())
                    .collect()
            },
            None => vec![],
        };

        Ok(bucket_names)
    }

    /// Return the bucket location (`Region`) for the given `bucket`.
    ///
    /// This method will properly handle the case of the `null` (empty) and
    /// `EU` location constraints, by replacing them with `us-east-1` and
    /// `eu-west-1` respectively.
    pub async fn get_bucket_location(&self, bucket: &str) -> Result<Region> {
        debug!("get_bucket_location for '{}'", bucket);

        let input = GetBucketLocationRequest {
            bucket: bucket.to_owned(),
        };

        let output   = self.client.get_bucket_location(input).await?;
        let location = output.location_constraint.expect("location");

        debug!("GetBucketLocation API returned '{}'", location);

        // Location constraints for sufficiently old buckets in S3 may not
        // quite meet expectations. These returns are badly documented and the
        // assumptions here are based on what the web console does.
        let location = match location.as_ref() {
            ""   => "us-east-1".to_string(),
            "EU" => "eu-west-1".to_string(),
            _    => location,
        };

        let location = Region::from_str(&location)?;

        Ok(location)
    }

    /// List object versions and filter according to `S3ObjectVersions`.
    ///
    /// This will be used when the size of `All` or `NonCurrent` objects is
    /// requested.
    async fn size_object_versions(&self, bucket: &str) -> Result<usize> {
        let mut next_key_marker        = None;
        let mut next_version_id_marker = None;
        let mut size                   = 0;

        // Loop until all object versions are processed
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
                        // Unwrap is hopefully safe, objects should always come
                        // with this.
                        let is_latest = v.is_latest.unwrap();

                        match self.object_versions {
                            S3ObjectVersions::All     => v.size,
                            S3ObjectVersions::Current => {
                                if is_latest {
                                    v.size
                                }
                                else {
                                    None
                                }
                            },
                            S3ObjectVersions::NonCurrent => {
                                if is_latest {
                                    None
                                }
                                else {
                                    v.size
                                }
                            },
                        }
                    })
                    .sum::<i64>() as usize;
            }

            // Check if we need to continue processing bucket output and store
            // the continuation tokens for the next loop if so.
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

    /// Return the size of current object versions in the bucket.
    ///
    /// This will be used when the size of `Current` objects is requested.
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

    /// A wrapper to call the appropriate bucket sizing function depending on
    /// the `S3ObjectVersions` configuration the `Client` was created with.
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
            bucket_name:     None,
            object_versions: versions,
            region:          Region::UsEast1,
        }
    }

    #[test]
    fn test_get_bucket_location() {
        let client = mock_client(
            Some("s3-get-bucket-location.xml"),
            S3ObjectVersions::Current,
        );

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::get_bucket_location(&client, "test-bucket"))
            .unwrap();

        let expected = Region::EuWest1;

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_get_bucket_location_eu() {
        let client = mock_client(
            Some("s3-get-bucket-location-eu.xml"),
            S3ObjectVersions::Current,
        );

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::get_bucket_location(&client, "test-bucket"))
            .unwrap();

        let expected = Region::EuWest1;

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_get_bucket_location_null() {
        let client = mock_client(
            Some("s3-get-bucket-location-null.xml"),
            S3ObjectVersions::Current,
        );

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::get_bucket_location(&client, "test-bucket"))
            .unwrap();

        let expected = Region::UsEast1;

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_list_buckets() {
        let client = mock_client(
            Some("s3-list-buckets.xml"),
            S3ObjectVersions::Current,
        );

        let mut ret = Runtime::new()
            .unwrap()
            .block_on(Client::list_buckets(&client))
            .unwrap();
        ret.sort();

        let expected: Vec<String> = vec![
            "a-bucket-name".into(),
            "another-bucket-name".into(),
        ];

        assert_eq!(ret, expected);
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
