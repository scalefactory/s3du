// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;
use crate::common::{
    BucketNames,
    BucketSizer,
};
use crate::s3::bucketlist::BucketList;
use crate::s3::client::Client;
use log::debug;
use rusoto_s3::S3;

#[async_trait]
impl BucketSizer for Client {
    // Return a list of S3 bucket names from CloudWatch.
    async fn list_buckets(&mut self) -> Result<BucketNames> {
        let bucket_list: BucketList = self.client.list_buckets().await?.into();
        let bucket_names            = bucket_list.bucket_names().to_owned();

        self.buckets = Some(bucket_list);

        Ok(bucket_names)
    }

    // Get the size of a given bucket
    async fn bucket_size(&self, bucket: &str) -> Result<usize> {
        debug!("bucket_size: Calculating size for '{}'", bucket);

        let size = self.size_objects(bucket).await?;

        debug!(
            "bucket_size: Calculated bucket size for '{}' is '{}'",
            bucket,
            size,
        );

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::S3ObjectVersions;
    use pretty_assertions::assert_eq;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
        ReadMockResponse,
    };
    use rusoto_s3::S3Client;
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
    fn test_list_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("s3-list-buckets.xml"),
            S3ObjectVersions::Current,
        );
        let mut ret = Runtime::new()
            .unwrap()
            .block_on(Client::list_buckets(&mut client))
            .unwrap();
        ret.sort();

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_bucket_size() {
        let client = mock_client(
            Some("s3-list-objects.xml"),
            S3ObjectVersions::Current,
        );

        let bucket = "test-bucket";
        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::bucket_size(&client, bucket))
            .unwrap();

        let expected = 33792;

        assert_eq!(ret, expected);
    }
}
