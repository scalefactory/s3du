// Implements the BucketSizer trait for CloudWatch Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    anyhow,
    Result,
};
use async_trait::async_trait;
use chrono::prelude::*;
use crate::common::{
    Bucket,
    Buckets,
    BucketSizer,
};
use log::debug;
use super::bucket_metrics::BucketMetrics;
use super::client::Client;

#[async_trait]
impl BucketSizer for Client {
    /// Return a list of S3 bucket names from CloudWatch.
    /// We also cache the returned metrics here, since we need to reference this
    /// elsewhere, and we don't want to have to query for it again.
    async fn buckets(&mut self) -> Result<Buckets> {
        let metrics: BucketMetrics = self.list_metrics().await?.into();

        let mut buckets = Buckets::new();

        for bucket in metrics.bucket_names() {
            let storage_types = metrics.storage_types(&bucket).to_owned();

            let bucket = Bucket {
                name:          bucket,
                region:        None,
                storage_types: Some(storage_types),
            };

            buckets.push(bucket);
        }

        Ok(buckets)
    }

    /// Get the size of a given bucket
    async fn bucket_size(&self, bucket: &Bucket) -> Result<usize> {
        let bucket_name = &bucket.name;

        debug!("bucket_size: Calculating size for '{}'", bucket_name);

        let mut size: usize = 0;

        let metric_statistics = self.get_metric_statistics(bucket).await?;
        for stats in metric_statistics {
            // If we don't get any datapoints, proceed to the next input.
            let mut datapoints = match stats.datapoints {
                Some(d) => d,
                None    => continue,
            };

            // It's possible that CloudWatch could return nothing. Return an
            // error in this case.
            if datapoints.is_empty() {
                return Err(
                    anyhow!("Failed to fetch any CloudWatch datapoints!")
                )
            };

            // We don't know which order datapoints will be in if we get more
            // than a single datapoint, so we must sort them.
            // We sort so that the latest datapoint is at index 0 of the vec.
            datapoints.sort_by(|a, b| {
                let a_timestamp: DateTime<Utc> = a.timestamp
                    .as_ref()
                    .expect("Couldn't unwrap a_timestamp")
                    .parse()
                    .expect("Couldn't parse a_timestamp");

                let b_timestamp: DateTime<Utc> = b.timestamp
                    .as_ref()
                    .expect("Couldn't unwrap b_timestamp")
                    .parse()
                    .expect("Couldn't parse b_timestamp");

                b_timestamp.cmp(&a_timestamp)
            });

            let datapoint = &datapoints[0];

            // BucketSizeBytes only supports Average, so this should be safe
            // to unwrap.
            let bytes = datapoint.average
                .expect("Could't unwrap average");

            // Add up the size of each storage type
            size += bytes as usize;

        }

        debug!(
            "bucket_size: Calculated bucket size for '{}' is '{}'",
            bucket_name,
            size,
        );

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rusoto_cloudwatch::CloudWatchClient;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
        ReadMockResponse,
    };
    use tokio::runtime::Runtime;

    // Create a mock CloudWatch client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Option<&str>,
    ) -> Client {
        let data = match data_file {
            None    => "".to_string(),
            Some(d) => MockResponseReader::read_response("test-data", d.into()),
        };

        let client = CloudWatchClient::new_with(
            MockRequestDispatcher::default().with_body(&data),
            MockCredentialsProvider,
            Default::default()
        );

        Client {
            client:      client,
            bucket_name: None,
        }
    }

    #[test]
    fn test_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
        );

        let buckets = Runtime::new()
            .unwrap()
            .block_on(Client::buckets(&mut client))
            .unwrap();

        let mut buckets: Vec<String> = buckets.iter()
            .map(|b| b.name.to_owned())
            .collect();

        buckets.sort();

        assert_eq!(buckets, expected);
    }

    #[test]
    fn test_bucket_size() {
        let client = mock_client(
            Some("cloudwatch-get-metric-statistics.xml"),
        );

        let storage_types = vec![
            "StandardStorage".into(),
        ];

        let bucket = Bucket {
            name:          "some-other-bucket-name".into(),
            region:        None,
            storage_types: Some(storage_types),
        };

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::bucket_size(&client, &bucket))
            .unwrap();

        let expected = 123456789;

        assert_eq!(ret, expected);
    }
}
