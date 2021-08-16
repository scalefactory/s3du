// Implements the S3 Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use aws_sdk_s3::{
    client::Client as S3Client,
    config::Config as S3Config,
    model::BucketLocationConstraint,
};
use crate::common::{
    BucketNames,
    ClientConfig,
    ObjectVersions,
    Region,
};
use log::debug;
use rayon::prelude::*;

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

        let s3config = S3Config::builder()
            .region(&region)
            .build();

        let s3client = S3Client::from_conf(s3config);

        Self {
            client:          s3client,
            bucket_name:     bucket_name,
            object_versions: config.object_versions,
            region:          region,
        }
    }

    /// Returns a list of bucket names.
    pub async fn list_buckets(&self) -> Result<BucketNames> {
        debug!("list_buckets");

        let output = self.client.list_buckets().send().await?;

        let bucket_names = if let Some(buckets) = output.buckets {
            buckets
                .par_iter()
                .filter_map(|b| b.name.to_owned())
                .collect()
        }
        else {
            Vec::new()
        };

        debug!("Found buckets: {:?}", bucket_names);

        Ok(bucket_names)
    }

    /// Return the bucket location (`Region`) for the given `bucket`.
    ///
    /// This method will properly handle the case of the `null` (empty) and
    /// `EU` location constraints, by replacing them with `us-east-1` and
    /// `eu-west-1` respectively.
    pub async fn get_bucket_location(&self, bucket: &str) -> Result<Region> {
        debug!("get_bucket_location for '{}'", bucket);

        let output = self.client.get_bucket_location()
            .bucket(bucket)
            .send()
            .await?;

        debug!("GetBucketLocation API returned '{:?}'", output);

        // Location constraints for sufficiently old buckets in S3 may not
        // quite meet expectations. These returns are badly documented and the
        // assumptions here are based on what the web console does.
        let location = match output.location_constraint {
            Some(BucketLocationConstraint::Eu) => "eu-west-1".to_string(),
            Some(location)                     => location.as_str().to_string(),
            None                               => "us-east-1".to_string(),
        };

        let location = Region::new().set_region(&location);

        debug!("Final location: {:?}", location);

        Ok(location)
    }

    /// Returns a `bool` indicating if we have access to the given `bucket` or
    /// not.
    pub async fn head_bucket(&self, bucket: &str) -> bool {
        debug!("head_bucket for '{}'", bucket);

        let output = self.client.head_bucket()
            .bucket(bucket)
            .send()
            .await;

        debug!("head_bucket output for '{}' -> '{:?}'", bucket, output);

        output.is_ok()
    }

    /// Returns a bool indicating if the region is a custom region
    pub fn is_custom_client_region(&self) -> bool {
        let constraint = BucketLocationConstraint::from(self.region.name());

        // We assume that any Unknown location constraint is a custom region
        matches!(constraint, BucketLocationConstraint::Unknown(_))
    }

    /// List in-progress multipart uploads
    async fn size_multipart_uploads(&self, bucket: &str) -> Result<usize> {
        let mut key_marker       = None;
        let mut size             = 0;
        let mut upload_id_marker = None;

        loop {
            let output = self.client.list_multipart_uploads()
                .bucket(bucket)
                .set_key_marker(key_marker)
                .set_upload_id_marker(upload_id_marker)
                .send()
                .await?;

            if let Some(uploads) = output.uploads {
                // No iterator here since we need to call an async method.
                for upload in uploads {
                    let key       = upload.key.expect("upload key");
                    let upload_id = upload.upload_id.expect("upload_id");

                    size += self.size_parts(bucket, &key, &upload_id).await?;
                }
            }

            if output.is_truncated {
                key_marker       = output.next_key_marker;
                upload_id_marker = output.next_upload_id_marker;
            }
            else {
                break;
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
            let output = self.client.list_object_versions()
                .bucket(bucket)
                .set_key_marker(next_key_marker)
                .set_version_id_marker(next_version_id_marker)
                .send()
                .await?;

            // Depending on which object versions we're paying attention to,
            // we may or may not filter here.
            if let Some(versions) = output.versions {
                size += versions
                    .par_iter()
                    .map(|v| {
                        // Here we take our object version selection into
                        // account.
                        //
                        // We return a size of 0 if we aren't interested in an
                        // object version.
                        //
                        // Multipart isn't handled here.
                        match self.object_versions {
                            ObjectVersions::All     => v.size,
                            ObjectVersions::Current => {
                                if v.is_latest {
                                    v.size
                                }
                                else {
                                    0
                                }
                            },
                            ObjectVersions::Multipart => unreachable!(),
                            ObjectVersions::NonCurrent => {
                                if v.is_latest {
                                    0
                                }
                                else {
                                    v.size
                                }
                            },
                        }
                    })
                    .sum::<i32>() as usize;
            }

            // Check if we need to continue processing bucket output and store
            // the continuation tokens for the next loop if so.
            if output.is_truncated {
                next_key_marker        = output.next_key_marker;
                next_version_id_marker = output.next_version_id_marker;
            }
            else {
                break;
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
            let output = self.client.list_objects_v2()
                .bucket(bucket)
                .set_continuation_token(continuation_token)
                .send()
                .await?;

            // Process the contents and add up the sizes
            if let Some(contents) = output.contents {
                size += contents
                    .par_iter()
                    .map(|o| o.size)
                    .sum::<i32>() as usize;
            }

            // If the output was truncated (Some(true)), we should have a
            // next_continuation_token.
            // If it wasn't, (Some(false) | None) we're done and can break.
            if output.is_truncated {
                continuation_token = output.next_continuation_token;
            }
            else {
                break;
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
                let mut size = 0;

                size += self.size_multipart_uploads(bucket).await?;
                size += self.size_object_versions(bucket).await?;

                Ok(size)
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
            let output = self.client.list_parts()
                .bucket(bucket)
                .key(key)
                .set_part_number_marker(part_number_marker)
                .upload_id(upload_id)
                .send()
                .await?;

            if let Some(parts) = output.parts {
                size += parts
                    .par_iter()
                    .map(|p| p.size)
                    .sum::<i32>() as usize;
            }

            if output.is_truncated {
                part_number_marker = output.next_part_number_marker;
            }
            else {
                break;
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_s3::Credentials;
    use pretty_assertions::assert_eq;
    use smithy_client::erase::DynConnector;
    use smithy_client::test_connection::TestConnection;
    use smithy_http::body::SdkBody;
    use std::fs;
    use std::path::Path;

    // Create a mock S3 client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Vec<&str>,
        versions:  ObjectVersions,
    ) -> Client {
        let creds = Credentials::from_keys(
            "ATESTCLIENT",
            "atestsecretkey",
            Some("atestsessiontoken".to_string()),
        );

        let conf = S3Config::builder()
            .credentials_provider(creds)
            .region(aws_sdk_s3::Region::new("eu-west-1"))
            .build();

        // Get a vec of events based on the given data_files
        let events = data_file
            .iter()
            .map(|d| {
                let path = Path::new("test-data").join(d);
                let data = fs::read_to_string(path).unwrap();

                // Events
                (
                    // Request
                    http::Request::builder()
                        .body(SdkBody::from("request body"))
                        .unwrap(),

                    // Response
                    http::Response::builder()
                        .status(200)
                        .body(SdkBody::from(data))
                        .unwrap(),
                )
            })
            .collect();

        let conn = TestConnection::new(events);
        let conn = DynConnector::new(conn);

        let client = S3Client::from_conf_conn(conf, conn);

        Client {
            client:          client,
            bucket_name:     None,
            object_versions: versions,
            region:          Region::new().set_region("eu-west-1"),
        }
    }

    // Create a mock client that returns a specific status code and empty
    // response body.
    fn mock_client_with_status(status: u16) -> Client {
        let creds = Credentials::from_keys(
            "ATESTCLIENT",
            "atestsecretkey",
            Some("atestsessiontoken".to_string()),
        );

        let conf = S3Config::builder()
            .credentials_provider(creds)
            .region(aws_sdk_s3::Region::new("eu-west-1"))
            .build();

        let events = vec![
            (
                // Request
                http::Request::builder()
                    .body(SdkBody::from("request body"))
                    .unwrap(),

                // Response
                http::Response::builder()
                    .status(status)
                    .body("response body")
                    .unwrap(),
            ),
        ];

        let conn = TestConnection::new(events);
        let conn = DynConnector::new(conn);

        let client = S3Client::from_conf_conn(conf, conn);

        Client {
            client:          client,
            bucket_name:     None,
            object_versions: ObjectVersions::Current,
            region:          Region::new().set_region("eu-west-1"),
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
            let ret    = client.head_bucket("test-bucket").await;

            assert_eq!(ret, expected);
        }
    }

    //#[tokio::test]
    //async fn test_get_bucket_location_err() {
    //    let client = mock_client(
    //        Some("s3-get-bucket-location-invalid.xml"),
    //        ObjectVersions::Current,
    //    );

    //    let ret = Client::get_bucket_location(&client, "test-bucket").await;
    //    println!("{:?}", ret);

    //    assert!(ret.is_err());
    //}

    #[tokio::test]
    async fn test_get_bucket_location_ok() {
        let client = mock_client(
            vec!["s3-get-bucket-location.xml"],
            ObjectVersions::Current,
        );

        let ret = client.get_bucket_location("test-bucket")
            .await
            .unwrap();

        let expected = Region::new().set_region("eu-west-1");

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_get_bucket_location_ok_eu() {
        let client = mock_client(
            vec!["s3-get-bucket-location-eu.xml"],
            ObjectVersions::Current,
        );

        let ret = client.get_bucket_location("test-bucket")
            .await
            .unwrap();

        let expected = Region::new().set_region("eu-west-1");

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_get_bucket_location_ok_null() {
        let client = mock_client(
            vec!["s3-get-bucket-location-null.xml"],
            ObjectVersions::Current,
        );

        let ret = client.get_bucket_location("test-bucket")
            .await
            .unwrap();

        let expected = Region::new().set_region("");

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_list_buckets() {
        let client = mock_client(
            vec!["s3-list-buckets.xml"],
            ObjectVersions::Current,
        );

        let mut ret = client.list_buckets().await.unwrap();
        ret.sort();

        let expected: Vec<String> = vec![
            "a-bucket-name".into(),
            "another-bucket-name".into(),
        ];

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_size_multipart_uploads() {
        let expected = 204_800;

        let data_files = vec![
            "s3-list-multipart-uploads.xml",
            "s3-list-parts.xml",
        ];

        let client = mock_client(
            data_files,
            ObjectVersions::Current,
        );

        let size = client.size_multipart_uploads("test-bucket").await.unwrap();

        assert_eq!(size, expected);
    }

    #[tokio::test]
    async fn test_size_objects() {
        let tests = vec![
            (
                ObjectVersions::All,
                805_532,
                vec![
                    "s3-list-multipart-uploads.xml",
                    "s3-list-parts.xml",
                    "s3-list-object-versions.xml",
                ],
            ),
            (
                ObjectVersions::Current,
                33_792,
                vec![
                    "s3-list-objects.xml",
                ],
            ),
            (
                ObjectVersions::Multipart,
                204_800,
                vec![
                    "s3-list-multipart-uploads.xml",
                    "s3-list-parts.xml",
                ],
            ),
            (
                ObjectVersions::NonCurrent,
                166_498,
                vec![
                    "s3-list-object-versions.xml",
                ],
            ),
        ];

        for test in tests {
            let versions      = test.0;
            let expected_size = test.1;
            let data_files    = test.2;

            let client = mock_client(
                data_files,
                versions,
            );

            let ret = client.size_objects("test-bucket")
                .await
                .unwrap();

            assert_eq!(ret, expected_size);
        }
    }

    #[tokio::test]
    async fn test_size_parts() {
        let client = mock_client(
            vec!["s3-list-parts.xml"],
            ObjectVersions::Current,
        );

        let ret = client.size_parts(
            "test-bucket",
            "test.zip",
            "abc123",
        ).await.unwrap();

        let expected = 1024 * 100 * 2;

        assert_eq!(ret, expected);
    }
}
