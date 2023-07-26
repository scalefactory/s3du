// Implement the BucketSizer trait for the s3::Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;
use crate::common::{
    Bucket,
    Buckets,
    BucketSizer,
};
use super::client::Client;
use tracing::debug;

#[async_trait]
impl BucketSizer for Client {
    /// Return `Buckets` discovered in S3.
    ///
    /// This list of buckets will also be filtered by the following:
    ///   - The `bucket` argument provided on the command line
    ///   - The `Region`, ensuring it's in our currently selected `--region`
    async fn buckets(&self) -> Result<Buckets> {
        debug!("buckets: Listing...");

        let mut bucket_names = self.list_buckets().await?;

        // If we were provided with a specific bucket name on the CLI, filter
        // out buckets that don't match.
        if let Some(bucket_name) = self.bucket_name.as_ref() {
            debug!("Filtering bucket list for '{}'", bucket_name);

            bucket_names.retain(|b| b == bucket_name);
        }

        let mut buckets = Buckets::new();

        for bucket in &bucket_names {
            debug!("Retrieving location for '{}'", bucket);

            let region = self.get_bucket_location(bucket).await?;

            // We can only ListBucket for the region our S3 client is in, so
            // we filter for that region here.
            if region == self.region || self.is_custom_client_region() {
                // If we don't have access to the bucket, skip it.
                if !self.head_bucket(bucket).await {
                    debug!("Access denied for '{}'", bucket);

                    continue;
                }

                let bucket = Bucket {
                    name:          bucket.into(),
                    region:        Some(region),
                    storage_types: None,
                };

                buckets.push(bucket);
            }
        }

        // Finally, we have a list of buckets that we should be able to get the
        // size for.
        Ok(buckets)
    }

    /// Return the size of `bucket`.
    async fn bucket_size(&self, bucket: &Bucket) -> Result<u64> {
        debug!("bucket_size: Calculating size for '{}'", bucket.name);

        let size = self.size_objects(&bucket.name).await?;

        debug!("bucket_size: size for '{}' is '{}'", bucket.name, size);

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_s3::client::Client as S3Client;
    use aws_sdk_s3::config::Config as S3Config;
    use aws_sdk_s3::config::Credentials;
    use aws_smithy_client::erase::DynConnector;
    use aws_smithy_client::test_connection::TestConnection;
    use aws_smithy_http::body::SdkBody;
    use crate::common::{
        ObjectVersions,
        Region,
    };
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;

    enum ResponseType<'a> {
        FromFile(&'a str),
        WithStatus(u16),
    }

    // Create a mock S3 client, returning the data from the specified
    // data_file.
    async fn mock_client<'a>(
        responses: Vec<ResponseType<'a>>,
        versions:  ObjectVersions,
    ) -> Client {
        // Get a vec of events based on the given data_files
        let events = responses
            .iter()
            .map(|r| {
                match r {
                    ResponseType::FromFile(file) => {
                        let path = Path::new("test-data").join(file);
                        let data = fs::read_to_string(path).unwrap();

                        (
                            http::Request::builder()
                                .body(SdkBody::from("request body"))
                                .unwrap(),

                            http::Response::builder()
                                .status(200)
                                .body(SdkBody::from(data))
                                .unwrap(),
                        )
                    },
                    ResponseType::WithStatus(status) => {
                        (
                            http::Request::builder()
                                .body(SdkBody::from("request body"))
                                .unwrap(),

                            http::Response::builder()
                                .status(*status)
                                .body(SdkBody::from(""))
                                .unwrap(),
                        )
                    },
                }
            })
            .collect();

        let conn = TestConnection::new(events);
        let conn = DynConnector::new(conn);

        let creds = Credentials::from_keys(
            "ATESTCLIENT",
            "atestsecretkey",
            Some("atestsessiontoken".to_string()),
        );

        let conf = S3Config::builder()
            .credentials_provider(creds)
            .http_connector(conn)
            .region(aws_sdk_s3::config::Region::new("eu-west-1"))
            .build();

        let client = S3Client::from_conf(conf);

        Client {
            client:          client,
            bucket_name:     None,
            object_versions: versions,
            region:          Region::new().set_region("eu-west-1"),
        }
    }

    #[tokio::test]
    async fn test_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let responses = vec![
            ResponseType::FromFile("s3-list-buckets.xml"),
            ResponseType::FromFile("s3-get-bucket-location.xml"),
            ResponseType::WithStatus(200),
            ResponseType::FromFile("s3-get-bucket-location.xml"),
            ResponseType::WithStatus(200),
        ];

        let client = mock_client(
            responses,
            ObjectVersions::Current,
        ).await;

        let buckets = client.buckets().await.unwrap();

        let mut buckets: Vec<String> = buckets.iter()
            .map(|b| b.name.to_owned())
            .collect();

        buckets.sort();

        assert_eq!(buckets, expected);
    }

    #[tokio::test]
    async fn test_bucket_size() {
        let client = mock_client(
            vec![ResponseType::FromFile("s3-list-objects.xml")],
            ObjectVersions::Current,
        ).await;

        let bucket = Bucket {
            name:          "test-bucket".into(),
            region:        None,
            storage_types: None,
        };

        let ret = client.bucket_size(&bucket).await.unwrap();

        let expected = 33792;

        assert_eq!(ret, expected);
    }
}
