// Implement the CloudWatch Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    Context,
    Result,
};
use aws_sdk_cloudwatch::client::Client as CloudWatchClient;
use aws_sdk_cloudwatch::operation::get_metric_statistics::GetMetricStatisticsOutput;
use aws_sdk_cloudwatch::primitives::DateTime;
use aws_sdk_cloudwatch::types::{
    Dimension,
    DimensionFilter,
    Metric,
    StandardUnit,
    Statistic,
};
use crate::common::{
    Bucket,
    ClientConfig,
};
use std::time::{
    Duration,
    SystemTime,
};
use tracing::debug;

const ONE_DAY: Duration = Duration::from_secs(86_400);

/// A `CloudWatch` `Client`
pub struct Client {
    /// The AWS SDK `CloudWatchClient`.
    pub client: CloudWatchClient,

    /// Bucket name that was selected, if any.
    pub bucket_name: Option<String>,
}

impl Client {
    /// Return a new `Client` with the given `ClientConfig`.
    pub async fn new(config: ClientConfig) -> Self {
        let bucket_name = config.bucket_name;
        let region      = config.region;

        debug!("new: Creating CloudWatchClient in region '{}'", region.name());

        let config = aws_config::from_env()
            .region(region.clone())
            .load()
            .await;

        let client = CloudWatchClient::new(&config);

        Self {
            client,
            bucket_name,
        }
    }

    /// Returns a `Vec` of `GetMetricStatisticsOutput` for the given `Bucket`.
    ///
    /// This returns a `Vec` because there is one `GetMetricStatisticsOutput`
    /// for each S3 bucket storage type that `CloudWatch` has statistics for.
    pub async fn get_metric_statistics(
        &self,
        bucket: &Bucket,
    ) -> Result<Vec<GetMetricStatisticsOutput>> {
        debug!("get_metric_statistics: Processing {:?}", bucket);

        // These are used repeatedly while looping, just prepare them once.
        let now = SystemTime::now();
        let start_time = DateTime::from(now - (ONE_DAY * 2));

        let period = i32::try_from(ONE_DAY.as_secs())
            .context("period")?;

        let storage_types = match &bucket.storage_types {
            Some(st) => st.clone(),
            None     => Vec::new(),
        };

        let mut outputs = Vec::new();

        for storage_type in storage_types {
            let dimensions = vec![
                Dimension::builder()
                    .name("BucketName")
                    .value(bucket.name.clone())
                    .build(),
                Dimension::builder()
                    .name("StorageType")
                    .value(storage_type.clone())
                    .build(),
            ];

            let input = self.client.get_metric_statistics()
                .end_time(DateTime::from(now))
                .metric_name("BucketSizeBytes")
                .namespace("AWS/S3")
                .period(period)
                .set_dimensions(Some(dimensions))
                .start_time(start_time)
                .statistics(Statistic::Average)
                .unit(StandardUnit::Bytes);

            debug!("{:?}", input);

            let output = input
                .send()
                .await?;

            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Get list of buckets with `BucketSizeBytes` metrics.
    ///
    /// An individual metric resembles the following:
    /// ```rust
    /// Metric {
    ///   metric_name: Some("BucketSizeBytes"),
    ///   namespace:   Some("AWS/S3")
    ///   dimensions:  Some([
    ///     Dimension {
    ///       name:  "StorageType",
    ///       value: "StandardStorage"
    ///     },
    ///     Dimension {
    ///       name:  "BucketName",
    ///       value: "some-bucket-name"
    ///     }
    ///   ]),
    /// }
    /// ```
    pub async fn list_metrics(&self) -> Result<Vec<Metric>> {
        println!("LISTING METRICS");
        debug!("list_metrics: Listing...");

        let mut metrics    = Vec::new();
        let mut next_token = None;

        // If we selected a bucket to list, filter for it here.
        let dimensions = match self.bucket_name.as_ref() {
            Some(bucket_name) => {
                let filter = DimensionFilter::builder()
                    .name("BucketName")
                    .value(bucket_name.clone())
                    .build();

                Some(vec![filter])
            },
            None => None,
        };

        // We loop until we've processed everything.
        loop {
            // Input for CloudWatch API
            let output = self.client.list_metrics()
                .namespace("AWS/S3")
                .metric_name("BucketSizeBytes")
                .set_dimensions(dimensions.clone())
                .set_next_token(next_token)
                .send()
                .await?;

            debug!("list_metrics: API returned: {:#?}", output);

            // If we get any metrics, append them to our vec
            let metric = output.metrics();
            metrics.append(&mut metric.to_vec());

            // If there was a next token, use it, otherwise the loop is done.
            match output.next_token() {
                Some(t) => next_token = Some(t.to_string()),
                None    => break,
            }
        }

        debug!("list_metrics: Metrics collection: {:#?}", metrics);

        Ok(metrics)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use aws_credential_types::Credentials;
    use aws_sdk_cloudwatch::config::Config as CloudWatchConfig;
    use aws_sdk_cloudwatch::primitives::DateTimeFormat;
    use aws_sdk_cloudwatch::types::{
        Datapoint,
        Dimension,
        Metric,
    };
    use aws_smithy_http_client::test_util::{
        ReplayEvent,
        StaticReplayClient,
    };
    use aws_smithy_types::body::SdkBody;
    use pretty_assertions::assert_eq;

    // Create a mock CloudWatch client, returning the data from the specified
    // data_file.
    fn mock_client(
        cbor_data: Vec<u8>,
    ) -> Client {
        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                http::Request::builder()
                    .body(SdkBody::empty())
                    .unwrap(),

                http::Response::builder()
                    .status(200)
                    .body(SdkBody::from(cbor_data))
                    .unwrap(),
                ),
        ]);

        let creds = Credentials::for_tests_with_session_token();

        let conf = CloudWatchConfig::builder()
            .behavior_version_latest()
            .credentials_provider(creds)
            .http_client(http_client)
            .region(aws_sdk_cloudwatch::config::Region::new("eu-west-1"))
            .build();

        let client = CloudWatchClient::from_conf(conf);

        Client {
            client,
            bucket_name: None,
        }
    }

    // CloudWatch tests in other modules import this too.
    pub fn cloudwatch_get_metric_statistics() -> Vec<u8> {
        let mut encoder = aws_smithy_cbor::Encoder::new(Vec::<u8>::new());

        let timestamp = DateTime::from_str(
            "2020-03-01T20:59:00Z",
            DateTimeFormat::DateTime,
        ).unwrap();

        let cbor = encoder
            .begin_map()
                .str("Label").str("BucketSizeBytes")
                .str("Datapoints").array(1)
                    .begin_map()
                        .str("Average").double(123_456_789.0)
                        .str("Timestamp").timestamp(&timestamp)
                        .str("Unit").str("Bytes")
                    .end() // end map 1
                // end array
            .end(); // end map

        cbor.clone().into_writer()
    }

    // CloudWatch tests in other modules import this too.
    pub fn cloudwatch_list_metrics() -> Vec<u8> {
        let mut encoder = aws_smithy_cbor::Encoder::new(Vec::<u8>::new());

        let cbor = encoder
            .begin_map()
                .str("Metrics").array(3)
                    .begin_map()
                        .str("MetricName").str("BucketSizeBytes")
                        .str("Namespace").str("AWS/S3")
                        .str("Dimensions").array(2)
                            .begin_map()
                                .str("Name").str("BucketName")
                                .str("Value").str("a-bucket-name")
                            .end()
                            .begin_map()
                                .str("Name").str("StorageType")
                                .str("Value").str("StandardStorage")
                            .end()
                        // end array
                    .end() // end map 1
                    .begin_map()
                        .str("MetricName").str("BucketSizeBytes")
                        .str("Namespace").str("AWS/S3")
                        .str("Dimensions").array(2)
                            .begin_map()
                                .str("Name").str("BucketName")
                                .str("Value").str("a-bucket-name")
                            .end()
                            .begin_map()
                                .str("Name").str("StorageType")
                                .str("Value").str("StandardIAStorage")
                            .end()
                        // end array
                    .end() // end map 2
                    .begin_map()
                        .str("MetricName").str("BucketSizeBytes")
                        .str("Namespace").str("AWS/S3")
                        .str("Dimensions").array(2)
                            .begin_map()
                                .str("Name").str("BucketName")
                                .str("Value").str("another-bucket-name")
                            .end()
                            .begin_map()
                                .str("Name").str("StorageType")
                                .str("Value").str("StandardStorage")
                            .end()
                        // end array
                    .end() // end map 3
                // end array
                .str("OwningAccounts").array(1)
                    .str("123456789012")
                // end array
            .end();

        cbor.clone().into_writer()
    }

    #[tokio::test]
    async fn test_get_metric_statistics() {
        let cbor = cloudwatch_get_metric_statistics();
        let client = mock_client(cbor);

        let storage_types = vec![
            "StandardStorage".into(),
        ];

        let bucket = Bucket {
            name:          "test-bucket".into(),
            region:        None,
            storage_types: Some(storage_types),
        };

        let ret = client.get_metric_statistics(&bucket)
            .await
            .unwrap();

        let timestamp = DateTime::from_str(
            "2020-03-01T20:59:00Z",
            DateTimeFormat::DateTime,
        ).unwrap();

        let datapoints = vec![
            Datapoint::builder()
                .average(123_456_789.0)
                .timestamp(timestamp)
                .unit(StandardUnit::Bytes)
                .build(),
        ];

        let expected = vec![
            GetMetricStatisticsOutput::builder()
                .set_datapoints(Some(datapoints))
                .set_label(Some("BucketSizeBytes".into()))
                .build(),
        ];

        assert_eq!(ret, expected);
    }

    #[tokio::test]
    async fn test_list_metrics() {
        let cbor = cloudwatch_list_metrics();
        let client = mock_client(cbor);
        let ret = client.list_metrics().await.unwrap();

        let expected = vec![
            Metric::builder()
                .metric_name("BucketSizeBytes")
                .namespace("AWS/S3")
                .set_dimensions(Some(vec![
                    Dimension::builder()
                        .name("BucketName")
                        .value("a-bucket-name")
                        .build(),

                    Dimension::builder()
                        .name("StorageType")
                        .value("StandardStorage")
                        .build(),
                ]))
                .build(),

            Metric::builder()
                .metric_name("BucketSizeBytes")
                .namespace("AWS/S3")
                .set_dimensions(Some(vec![
                    Dimension::builder()
                        .name("BucketName")
                        .value("a-bucket-name")
                        .build(),

                    Dimension::builder()
                        .name("StorageType")
                        .value("StandardIAStorage")
                        .build(),
                ]))
                .build(),

            Metric::builder()
                .metric_name("BucketSizeBytes")
                .namespace("AWS/S3")
                .set_dimensions(Some(vec![
                    Dimension::builder()
                        .name("BucketName")
                        .value("another-bucket-name")
                        .build(),

                    Dimension::builder()
                        .name("StorageType")
                        .value("StandardStorage")
                        .build(),
                ]))
                .build(),
        ];

        assert_eq!(ret, expected);
    }
}
