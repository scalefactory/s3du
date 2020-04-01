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
    async fn buckets(&mut self) -> Result<Buckets> {
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
            if region == self.region {
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
    use crate::common::ObjectVersions;
    use pretty_assertions::assert_eq;
    use rusoto_core::Region;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
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
            region:          Region::UsEast1,
        }
    }

    // This test is currently ignored as we cannot easily mock multiple
    // requests at the moment. Issues #1671 and PR #1685 should solve this.
    #[tokio::test]
    #[ignore]
    async fn test_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("s3-list-buckets.xml"),
            ObjectVersions::Current,
        );

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
