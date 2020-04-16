// Implements the S3 Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use crate::common::{
    BucketNames,
    ClientConfig,
    ObjectVersions,
};
use log::debug;
use rusoto_core::Region;
use rusoto_s3::{
    HeadBucketRequest,
    GetBucketLocationRequest,
    ListMultipartUploadsRequest,
    ListObjectsV2Request,
    ListObjectVersionsRequest,
    ListPartsRequest,
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
    pub object_versions: ObjectVersions,

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
            object_versions: config.object_versions,
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
            None => Vec::new(),
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

    /// Returns a `bool` indicating if we have access to the given `bucket` or
    /// not.
    pub async fn head_bucket(&self, bucket: &str) -> bool {
        debug!("head_bucket for '{}'", bucket);

        let input = HeadBucketRequest {
            bucket: bucket.into(),
        };

        let output = self.client.head_bucket(input).await;

        debug!("head_bucket output for '{}' -> '{:?}'", bucket, output);

        match output {
            Ok(_)  => true,
            Err(_) => false,
        }
    }

    /// List in-progress multipart uploads
    async fn size_multipart_uploads(&self, bucket: &str) -> Result<usize> {
        let mut key_marker       = None;
        let mut size             = 0;
        let mut upload_id_marker = None;

        loop {
            let input = ListMultipartUploadsRequest {
                bucket:           bucket.into(),
                key_marker:       key_marker.to_owned(),
                upload_id_marker: upload_id_marker.to_owned(),
                ..Default::default()
            };

            let output = self.client.list_multipart_uploads(input).await?;

            if let Some(uploads) = output.uploads {
                // No iterator here since we need to call an async method.
                for upload in uploads {
                    let key       = upload.key.expect("upload key");
                    let upload_id = upload.upload_id.expect("upload_id");

                    size += self.size_parts(bucket, &key, &upload_id).await?;
                }
            }

            match output.is_truncated {
                Some(true) => {
                    key_marker       = output.next_key_marker;
                    upload_id_marker = output.next_upload_id_marker;
                },
                _ => break,
            }
        }

        Ok(size)
    }

    /// List object versions and filter according to `ObjectVersions`.
    ///
    /// This will be used when the size of `All` or `NonCurrent` objects is
    /// requested.
    async fn size_object_versions(&self, bucket: &str) -> Result<usize> {
        debug!("size_object_versions for '{}'", bucket);

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
                        // Multipart isn't handled here.
                        let is_latest = v.is_latest.unwrap();

                        match self.object_versions {
                            ObjectVersions::All     => v.size,
                            ObjectVersions::Current => {
                                if is_latest {
                                    v.size
                                }
                                else {
                                    None
                                }
                            },
                            ObjectVersions::Multipart => unimplemented!(),
                            ObjectVersions::NonCurrent => {
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
        debug!("size_current_objects for '{}'", bucket);

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
    /// the `ObjectVersions` configuration the `Client` was created with.
    pub async fn size_objects(&self, bucket: &str) -> Result<usize> {
        debug!("size_objects: '{}' with {:?}", bucket, self.object_versions);

        match self.object_versions {
            ObjectVersions::All => {
                self.size_object_versions(bucket).await
            },
            ObjectVersions::Current => {
                self.size_current_objects(bucket).await
            },
            ObjectVersions::Multipart => {
                self.size_multipart_uploads(bucket).await
            },
            ObjectVersions::NonCurrent => {
                self.size_object_versions(bucket).await
            },
        }
    }

    /// List parts of an in-progress multipart upload
    async fn size_parts(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<usize> {
        let mut part_number_marker = None;
        let mut size               = 0;

        loop {
            let input = ListPartsRequest {
                bucket:             bucket.into(),
                key:                key.into(),
                part_number_marker: part_number_marker.to_owned(),
                upload_id:          upload_id.into(),
                ..Default::default()
            };

            let output = self.client.list_parts(input).await?;

            if let Some(parts) = output.parts {
                size += parts
                    .iter()
                    .filter_map(|p| p.size)
                    .sum::<i64>() as usize;
            }

            match output.is_truncated {
                Some(true) => {
                    part_number_marker = output.next_part_number_marker;
                },
                _ => break,
            }
        }

        Ok(size)
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

    // Create a mock S3 client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Option<&str>,
        versions:  ObjectVersions,
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

    // Create a mock client that returns a specific status code and empty
    // response body.
    fn mock_client_with_status(status: u16) -> Client {
        let dispatcher = MockRequestDispatcher::with_status(status);

        let client = S3Client::new_with(
            dispatcher,
            MockCredentialsProvider,
            Default::default()
        );

        Client {
            client:          client,
            bucket_name:     None,
            object_versions: ObjectVersions::Current,
            region:          Region::UsEast1,
        }
    }

    #[tokio::test]
    async fn test_head_bucket() {
        let tests = vec![
            (200, true),
            (403, false),
            (404, false),
        ];

        for test in tests {
            let status_code: u16 = test.0;
            let expected         = test.1;

            let client = mock_client_with_status(status_code);
            let ret    = Client::head_bucket(&client, "test-bucket").await;

            assert_eq!(ret, expected);
        }
    }

    #[tokio::test]
    async fn test_get_bucket_location_err() {
        let client = mock_client(
            Some("s3-get-bucket-location-invalid.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::get_bucket_location(&client, "test-bucket").await;

        assert!(ret.is_err());
    }

    #[tokio::test]
    async fn test_get_bucket_location_ok() {
        let client = mock_client(
            Some("s3-get-bucket-location.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::get_bucket_location(&client, "test-bucket")
            .await
            .unwrap();

        let expected = Region::EuWest1;

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_get_bucket_location_ok_eu() {
        let client = mock_client(
            Some("s3-get-bucket-location-eu.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::get_bucket_location(&client, "test-bucket")
            .await
            .unwrap();

        let expected = Region::EuWest1;

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_get_bucket_location_ok_null() {
        let client = mock_client(
            Some("s3-get-bucket-location-null.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::get_bucket_location(&client, "test-bucket")
            .await
            .unwrap();

        let expected = Region::UsEast1;

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_list_buckets() {
        let client = mock_client(
            Some("s3-list-buckets.xml"),
            ObjectVersions::Current,
        );

        let mut ret = Client::list_buckets(&client).await.unwrap();
        ret.sort();

        let expected: Vec<String> = vec![
            "a-bucket-name".into(),
            "another-bucket-name".into(),
        ];

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_size_objects_current() {
        let mut client = mock_client(
            Some("s3-list-objects.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::size_objects(&mut client, "test-bucket")
            .await
            .unwrap();

        let expected = 33_792;

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_size_objects_all_noncurrent() {
        // Current expects 0 here because our mock client won't return any
        // objects. This is handled in another test.
        let tests = vec![
            (ObjectVersions::All,        600_732),
            (ObjectVersions::Current,    0),
            (ObjectVersions::NonCurrent, 166_498),
        ];

        for test in tests {
            let versions      = test.0;
            let expected_size = test.1;

            let mut client = mock_client(
                Some("s3-list-object-versions.xml"),
                versions,
            );

            let ret = Client::size_objects(&mut client, "test-bucket")
                .await
                .unwrap();

            assert_eq!(ret, expected_size);
        }
    }

    #[tokio::test]
    async fn test_size_parts() {
        let client = mock_client(
            Some("s3-list-parts.xml"),
            ObjectVersions::Current,
        );

        let ret = Client::size_parts(
            &client,
            "test-bucket",
            "test.zip",
            "abc123",
        ).await.unwrap();

        let expected = 1024 * 100 * 2;

        assert_eq!(ret, expected);
    }
}
