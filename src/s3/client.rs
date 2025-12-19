// Implements the S3 Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    Context,
    Result,
};
use aws_sdk_s3::client::Client as S3Client;
use aws_sdk_s3::types::{
    BucketLocationConstraint,
    Object,
    Part,
};
use crate::common::{
    BucketNames,
    ClientConfig,
    ObjectVersions,
    Region,
};
use rayon::prelude::*;
use tracing::debug;

/// The S3 `Client`.
pub struct Client {
    /// The AWS SDK `S3Client`.
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
    pub async fn new(config: ClientConfig) -> Self {
        let region = config.region;

        debug!(
            "new: Creating S3Client in region '{}'",
            region.name(),
        );

        let s3config = aws_config::from_env()
            .region(region.clone());

        let s3config = if let Some(endpoint) = config.endpoint {
            s3config.endpoint_url(endpoint)
        }
        else {
            s3config
        };

        let s3config = s3config
            .load()
            .await;

        let client = S3Client::new(&s3config);

        Self {
            client,
            region,
            bucket_name:     config.bucket_name,
            object_versions: config.object_versions,
        }
    }

    /// Returns a list of bucket names.
    pub async fn list_buckets(&self) -> Result<BucketNames> {
        debug!("list_buckets");

        let output = self.client.list_buckets().send().await?;

        let bucket_names = output.buckets()
            .par_iter()
            .filter_map(|bucket| bucket.name.clone())
            .collect();

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
        let location = match output.location_constraint() {
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
        // We assume that any unknown location constraint is a custom region
        BucketLocationConstraint::values()
            .contains(&self.region.name())
    }

    /// List in-progress multipart uploads
    async fn size_multipart_uploads(&self, bucket: &str) -> Result<u64> {
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

            // No iterator here since we need to call an async method.
            for upload in output.uploads() {
                let key       = upload.key().expect("upload key");
                let upload_id = upload.upload_id().expect("upload_id");

                size += self.size_parts(bucket, key, upload_id).await?;
            }

            if matches!(output.is_truncated(), Some(true)) {
                key_marker = output.next_key_marker()
                    .map(ToOwned::to_owned);

                upload_id_marker = output.next_upload_id_marker()
                    .map(ToOwned::to_owned);
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
    async fn size_object_versions(&self, bucket: &str) -> Result<u64> {
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
            let version_size = output.versions()
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
                        ObjectVersions::All     => v.size().unwrap_or(0),
                        ObjectVersions::Current => {
                            if v.is_latest() == Some(true) {
                                v.size().unwrap_or(0)
                            }
                            else {
                                0
                            }
                        },
                        ObjectVersions::Multipart => unreachable!(),
                        ObjectVersions::NonCurrent => {
                            if v.is_latest() == Some(true) {
                                0
                            }
                            else {
                                v.size().unwrap_or(0)
                            }
                        },
                    }
                })
                .sum::<i64>();

            size += u64::try_from(version_size)
                .context("version size")?;

            // Check if we need to continue processing bucket output and store
            // the continuation tokens for the next loop if so.
            if matches!(output.is_truncated(), Some(true)) {
                next_key_marker = output.next_key_marker()
                    .map(ToOwned::to_owned);

                next_version_id_marker = output.next_version_id_marker()
                    .map(ToOwned::to_owned);
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
    async fn size_current_objects(&self, bucket: &str) -> Result<u64> {
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
            let object_size = output.contents()
                .par_iter()
                .filter_map(Object::size)
                .sum::<i64>();

            size += u64::try_from(object_size)
                .context("object size")?;

            // If the output was truncated (Some(true)), we should have a
            // next_continuation_token.
            // If it wasn't, (Some(false) | None) we're done and can break.
            if matches!(output.is_truncated(), Some(true)) {
                continuation_token = output.next_continuation_token()
                    .map(ToOwned::to_owned);
            }
            else {
                break;
            }
        }

        Ok(size)
    }

    /// A wrapper to call the appropriate bucket sizing function depending on
    /// the `ObjectVersions` configuration the `Client` was created with.
    pub async fn size_objects(&self, bucket: &str) -> Result<u64> {
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
    ) -> Result<u64> {
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

            let part_sizes = output.parts()
                .par_iter()
                .filter_map(Part::size)
                .sum::<i64>();

            size += u64::try_from(part_sizes)
                .context("part sizes")?;

            if output.is_truncated() == Some(true) {
                part_number_marker = output.next_part_number_marker()
                    .map(ToOwned::to_owned);
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
    use aws_credential_types::Credentials;
    use aws_sdk_s3::config::Config as S3Config;
    use aws_smithy_http_client::test_util::{
        ReplayEvent,
        StaticReplayClient,
    };
    use aws_smithy_types::body::SdkBody;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;

    // Create a mock S3 client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: &[&str],
        versions:  ObjectVersions,
    ) -> Client {
        // Get a vec of events based on the given data_files
        let events = data_file
            .iter()
            .map(|d| {
                let path = Path::new("test-data").join(d);
                let data = fs::read_to_string(path).unwrap();

                // Events
                ReplayEvent::new(
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

        let http_client = StaticReplayClient::new(events);

        let creds = Credentials::for_tests_with_session_token();

        let conf = S3Config::builder()
            .behavior_version_latest()
            .credentials_provider(creds)
            .http_client(http_client)
            .region(aws_sdk_s3::config::Region::new("eu-west-1"))
            .build();

        let client = S3Client::from_conf(conf);

        Client {
            client,
            bucket_name: None,
            object_versions: versions,
            region: Region::new().set_region("eu-west-1"),
        }
    }

    // Create a mock client that returns a specific status code and empty
    // response body.
    fn mock_client_with_status(status: u16) -> Client {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                // Request
                http::Request::builder()
                    .body(SdkBody::from("request body"))
                    .unwrap(),

                // Response
                http::Response::builder()
                    .status(status)
                    .body(SdkBody::from("response body"))
                    .unwrap(),
            ),
        ]);

        let creds = Credentials::for_tests_with_session_token();

        let conf = S3Config::builder()
            .behavior_version_latest()
            .credentials_provider(creds)
            .http_client(http_client)
            .region(aws_sdk_s3::config::Region::new("eu-west-1"))
            .build();

        let client = S3Client::from_conf(conf);

        Client {
            client,
            bucket_name: None,
            object_versions: ObjectVersions::Current,
            region: Region::new().set_region("eu-west-1"),
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
            &["s3-get-bucket-location.xml"],
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
            &["s3-get-bucket-location-eu.xml"],
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
            &["s3-get-bucket-location-null.xml"],
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
            &["s3-list-buckets.xml"],
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
            &data_files,
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
                &data_files,
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
            &["s3-list-parts.xml"],
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
