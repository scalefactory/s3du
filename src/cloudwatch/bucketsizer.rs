// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    anyhow,
    Result,
};
use async_trait::async_trait;
use chrono::prelude::*;
use chrono::Duration;
use log::debug;
use rusoto_cloudwatch::{
    CloudWatch,
    Dimension,
    GetMetricStatisticsInput,
};
use crate::common::{
    BucketNames,
    BucketSizer,
};

use super::bucketmetrics::*;
use super::client::*;

const S3_BUCKETSIZEBYTES: &str = "BucketSizeBytes";
const S3_NAMESPACE: &str = "AWS/S3";

#[async_trait]
impl BucketSizer for Client {
    // Return a list of S3 bucket names from CloudWatch.
    async fn list_buckets(&mut self) -> Result<BucketNames> {
        let metrics: BucketMetrics = self.list_metrics().await?.into();
        let bucket_names           = metrics.bucket_names();

        self.metrics = Some(metrics);

        Ok(bucket_names)
    }

    // Get the size of a given bucket
    async fn bucket_size(&self, bucket: &str) -> Result<usize> {
        debug!("bucket_size: Calculating size for '{}'", bucket);

        let mut size: usize = 0;

        // We need to know which storage types are available for a bucket.
        let metrics = match &self.metrics {
            Some(m) => Ok(m),
            None    => Err(anyhow!("No bucket metrics")),
        }?;
        let storage_types = metrics.storage_types(bucket);

        debug!(
            "bucket_size: Found storage types '{:?}' for '{}'",
            storage_types,
            bucket,
        );

        // Get the time now so we can select the last 24 hours of metrics.
        let now: DateTime<Utc> = Utc::now();
        let one_day = Duration::days(1);

        // Create queries for each bucket storage type.
        let iter = storage_types.iter();
        let inputs: Vec<GetMetricStatisticsInput> = iter.map(|st| {
            // Dimensions for bucket selection
            let dimensions = vec![
                Dimension {
                    name:  "BucketName".into(),
                    value: bucket.into(),
                },
                Dimension {
                    name:  "StorageType".into(),
                    value: st.into(),
                },
            ];

            // Actual query
            GetMetricStatisticsInput {
                dimensions:  Some(dimensions),
                end_time:    self.iso8601(now),
                metric_name: S3_BUCKETSIZEBYTES.into(),
                namespace:   S3_NAMESPACE.into(),
                period:      one_day.num_seconds(),
                start_time:  self.iso8601(now - one_day),
                statistics:  Some(vec!["Average".into()]),
                unit:        Some("Bytes".into()),
                ..Default::default()
            }
        })
        .collect();

        // Perform a query for each bucket storage type
        for input in inputs {
            debug!(
                "bucket_size: Performing API call for input: {:#?}",
                input,
            );

            let output = self.client.get_metric_statistics(input).await?;

            debug!("bucket_size: API returned: {:#?}", output);

            // If we don't get any datapoints, proceed to the next input
            let datapoints = match output.datapoints {
                Some(d) => d,
                None    => continue,
            };

            // We only use 24h of data, so there should only ever be one
            // datapoint.
            let datapoint = &datapoints[0];

            // BucketSizeBytes only supports Average, so this should be safe
            // to unwrap.
            let bytes = datapoint.average.unwrap();

            // Add up the size of each storage type
            size += bytes as usize;
        }

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
    use pretty_assertions::assert_eq;
    use rusoto_cloudwatch::{
        CloudWatchClient,
        Dimension,
        Metric,
    };
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
        metrics: Option<BucketMetrics>,
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
            client:  client,
            metrics: metrics,
        }
    }

    // Metrics used in the tests
    fn get_metrics() -> Vec<Metric> {
        vec![
            Metric {
                metric_name: Some("BucketSizeBytes".into()),
                namespace:   Some("AWS/S3".into()),
                dimensions:  Some(vec![
                    Dimension {
                        name:  "StorageType".into(),
                        value: "StandardStorage".into(),
                    },
                    Dimension {
                        name:  "BucketName".into(),
                        value: "some-bucket-name".into(),
                    },
                    Dimension {
                        name:  "StorageType".into(),
                        value: "StandardIAStorage".into(),
                    },
                ]),
            },
            Metric {
                metric_name: Some("BucketSizeBytes".into()),
                namespace:   Some("AWS/S3".into()),
                dimensions:  Some(vec![
                    Dimension {
                        name:  "StorageType".into(),
                        value: "StandardStorage".into(),
                    },
                    Dimension {
                        name:  "BucketName".into(),
                        value: "some-other-bucket-name".into(),
                    },
                ]),
            },
        ]
    }

    #[test]
    fn test_list_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
            None,
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
        let metrics = get_metrics();
        let metrics: BucketMetrics = metrics.into();

        let client = mock_client(
            Some("cloudwatch-get-metric-statistics.xml"),
            Some(metrics),
        );

        let bucket = "some-other-bucket-name";
        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::bucket_size(&client, bucket))
            .unwrap();

        let expected = 123456789;

        assert_eq!(ret, expected);
    }
}
