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
use log::debug;
use super::client::Client;

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

            let region = self.get_bucket_location(&bucket).await?;

            // We can only ListBucket for the region our S3 client is in, so
            // we filter for that region here.
            if region == self.region || self.is_custom_client_region() {
                // If we don't have access to the bucket, skip it.
                if !self.head_bucket(&bucket).await {
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
    async fn bucket_size(&self, bucket: &Bucket) -> Result<usize> {
        debug!("bucket_size: Calculating size for '{}'", bucket.name);

        let size = self.size_objects(&bucket.name).await?;

        debug!("bucket_size: size for '{}' is '{}'", bucket.name, size);

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        ObjectVersions,
        Region,
    };
    use pretty_assertions::assert_eq;
    use rusoto_core::Region as RusotoRegion;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
        MultipleMockRequestDispatcher,
        ReadMockResponse,
    };
    use rusoto_s3::S3Client;

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
            region:          Region::new(),
        }
    }

    // Return a MockRequestDispatcher with a body given by the data_file.
    fn dispatcher_with_body(data_file: &str) -> MockRequestDispatcher {
        let data = MockResponseReader::read_response("test-data", data_file);

        MockRequestDispatcher::default().with_body(&data)
    }

    #[tokio::test]
    async fn test_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        // This ListBuckets request returns two buckets, so we have to mock two
        // pairs of GetBucketLocation and HeadBucket responses.
        let mock = MultipleMockRequestDispatcher::new(vec![
            dispatcher_with_body("s3-list-buckets.xml"),
            dispatcher_with_body("s3-get-bucket-location.xml"),
            MockRequestDispatcher::with_status(200),
            dispatcher_with_body("s3-get-bucket-location.xml"),
            MockRequestDispatcher::with_status(200),
        ]);

        let s3client = S3Client::new_with(
            mock,
            MockCredentialsProvider,
            RusotoRegion::UsEast1,
        );

        let mut client = Client {
            client:          s3client,
            bucket_name:     None,
            object_versions: ObjectVersions::Current,
            region:          Region::new(),
        };

        let buckets = Client::buckets(&mut client).await.unwrap();

        let mut buckets: Vec<String> = buckets.iter()
            .map(|b| b.name.to_owned())
            .collect();

        buckets.sort();

        assert_eq!(buckets, expected);
    }

    #[tokio::test]
    async fn test_bucket_size() {
        let client = mock_client(
            Some("s3-list-objects.xml"),
            ObjectVersions::Current,
        );

        let bucket = Bucket {
            name:          "test-bucket".into(),
            region:        None,
            storage_types: None,
        };

        let ret = Client::bucket_size(&client, &bucket).await.unwrap();

        let expected = 33792;

        assert_eq!(ret, expected);
    }
}
