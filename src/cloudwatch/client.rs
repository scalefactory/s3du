// Implement the CloudWatch Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use aws_config::meta::region::ProvideRegion;
use aws_sdk_cloudwatch::{
    client::Client as CloudWatchClient,
    config::Config as CloudWatchConfig,
};
use aws_sdk_cloudwatch::model::{
    Dimension,
    DimensionFilter,
    Metric,
    StandardUnit,
    Statistic,
};
use aws_sdk_cloudwatch::output::GetMetricStatisticsOutput;
use chrono::prelude::*;
use chrono::Duration;
use crate::common::{
    Bucket,
    ClientConfig,
};
use log::debug;

/// A CloudWatch `Client`
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

        let config = CloudWatchConfig::builder()
            .region(region.region().await)
            .build();

        let client = CloudWatchClient::from_conf(config);

        Self {
            client:      client,
            bucket_name: bucket_name,
        }
    }

    /// Returns a `Vec` of `GetMetricStatisticsOutput` for the given `Bucket`.
    ///
    /// This returns a `Vec` because there is one `GetMetricStatisticsOutput`
    /// for each S3 bucket storage type that CloudWatch has statistics for.
    pub async fn get_metric_statistics(
        &self,
        bucket: &Bucket,
    ) -> Result<Vec<GetMetricStatisticsOutput>> {
        debug!("get_metric_statistics: Processing {:?}", bucket);

        // These are used repeatedly while looping, just prepare them once.
        let now: DateTime<Utc> = Utc::now();
        let one_day            = Duration::days(1);
        let period             = one_day.num_seconds() as i32;
        let start_time         = (now - (one_day * 2)).into();

        let storage_types = match &bucket.storage_types {
            Some(st) => st.to_owned(),
            None     => Vec::new(),
        };

        let mut outputs = Vec::new();

        for storage_type in storage_types {
            let dimensions = vec![
                Dimension::builder()
                    .name("BucketName")
                    .value(bucket.name.to_owned())
                    .build(),
                Dimension::builder()
                    .name("StorageType")
                    .value(storage_type.to_owned())
                    .build(),
            ];

            let input = self.client.get_metric_statistics()
                .end_time(now.into())
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
        debug!("list_metrics: Listing...");

        let mut metrics    = Vec::new();
        let mut next_token = None;

        // If we selected a bucket to list, filter for it here.
        let dimensions = match self.bucket_name.as_ref() {
            Some(bucket_name) => {
                let filter = DimensionFilter::builder()
                    .name("BucketName")
                    .value(bucket_name.to_owned())
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
            if let Some(m) = output.metrics {
                metrics.append(&mut m.clone());
            }

            // If there was a next token, use it, otherwise the loop is done.
            match output.next_token {
                Some(t) => next_token = Some(t),
                None    => break,
            };
        }

        debug!("list_metrics: Metrics collection: {:#?}", metrics);

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatch::Credentials;
    use aws_sdk_cloudwatch::model::{
        Datapoint,
        Dimension,
        Metric,
    };
    use aws_smithy_client::erase::DynConnector;
    use aws_smithy_client::test_connection::TestConnection;
    use aws_smithy_http::body::SdkBody;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;

    // Create a mock CloudWatch client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Option<&str>,
    ) -> Client {
        let creds = Credentials::from_keys(
            "ATESTCLIENT",
            "atestsecretkey",
            Some("atestsessiontoken".to_string()),
        );

        let conf = CloudWatchConfig::builder()
            .credentials_provider(creds)
            .region(aws_sdk_cloudwatch::Region::new("eu-west-1"))
            .build();

        let data = match data_file {
            None    => "".to_string(),
            Some(d) => {
                let path = Path::new("test-data").join(d);
                fs::read_to_string(path).unwrap()
            },
        };

        let events = vec![
            (
                http::Request::builder()
                    .body(SdkBody::from("request body"))
                    .unwrap(),

                http::Response::builder()
                    .status(200)
                    .body(SdkBody::from(data))
                    .unwrap(),
            ),
        ];

        let conn = TestConnection::new(events);
        let conn = DynConnector::new(conn);

        let client = CloudWatchClient::from_conf_conn(conf, conn);

        Client {
            client:      client,
            bucket_name: None,
        }
    }

    #[tokio::test]
    async fn test_get_metric_statistics() {
        let client = mock_client(
            Some("cloudwatch-get-metric-statistics.xml"),
        );

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

        let timestamp = DateTime::parse_from_rfc3339("2020-03-01T20:59:00Z")
            .unwrap();

        let datapoints = vec![
            Datapoint::builder()
                .average(123456789.0)
                .timestamp(timestamp.into())
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
        let client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
        );

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
