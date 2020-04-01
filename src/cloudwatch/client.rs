// Implement the CloudWatch Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use chrono::prelude::*;
use chrono::Duration;
use crate::common::{
    Bucket,
    ClientConfig,
};
use log::debug;
use rusoto_cloudwatch::{
    CloudWatch,
    CloudWatchClient,
    Dimension,
    DimensionFilter,
    GetMetricStatisticsInput,
    GetMetricStatisticsOutput,
    ListMetricsInput,
    Metric,
};

/// A CloudWatch `Client`
pub struct Client {
    /// The Rusoto `CloudWatchClient`.
    pub client: CloudWatchClient,

    /// Bucket name that was selected, if any.
    pub bucket_name: Option<String>,
}

impl Client {
    /// Return a new `Client` with the given `ClientConfig`.
    pub fn new(config: ClientConfig) -> Self {
        let bucket_name = config.bucket_name;
        let region      = config.region;

        debug!(
            "new: Creating CloudWatchClient in region '{}'",
            region.name(),
        );

        let client = CloudWatchClient::new(region);

        Client {
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
        let now: DateTime<Utc> = Utc::now();
        let one_day            = Duration::days(1);

        let storage_types = match &bucket.storage_types {
            Some(st) => st.to_owned(),
            None     => vec![],
        };

        let inputs: Vec<GetMetricStatisticsInput> = storage_types.iter()
            .map(|storage_type| {
                let dimensions = vec![
                    Dimension {
                        name:  "BucketName".into(),
                        value: bucket.name.to_owned(),
                    },
                    Dimension {
                        name:  "StorageType".into(),
                        value: storage_type.to_owned(),
                    },
                ];

                GetMetricStatisticsInput {
                    dimensions: Some(dimensions),
                    end_time:   self.iso8601(now),
                    metric_name: "BucketSizeBytes".into(),
                    namespace:   "AWS/S3".into(),
                    period:      one_day.num_seconds(),
                    start_time:  self.iso8601(now - (one_day * 2)),
                    statistics:  Some(vec!["Average".into()]),
                    unit:        Some("Bytes".into()),
                    ..Default::default()
                }
            })
            .collect();

        let mut outputs = vec![];

        for input in inputs {
            let output = self.client.get_metric_statistics(input).await?;
            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Return an ISO8601 formatted timestamp suitable for
    /// `GetMetricsStatisticsInput`.
    pub fn iso8601(&self, dt: DateTime<Utc>) -> String {
        dt.to_rfc3339_opts(SecondsFormat::Secs, true)
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

        let mut metrics    = vec![];
        let mut next_token = None;

        // If we selected a bucket to list, filter for it here.
        let dimensions = match self.bucket_name.as_ref() {
            Some(bucket_name) => {
                let filter = DimensionFilter {
                    name: "BucketName".into(),
                    value: Some(bucket_name.to_owned()),
                };

                Some(vec![filter])
            },
            None => None,
        };

        // We loop until we've processed everything.
        loop {
            // Input for CloudWatch API
            let list_metrics_input = ListMetricsInput {
                dimensions:  dimensions.clone(),
                metric_name: Some("BucketSizeBytes".into()),
                namespace:   Some("AWS/S3".into()),
                next_token:  next_token,
                ..Default::default()
            };

            // Call the API
            let output = self.client.list_metrics(list_metrics_input).await?;

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
    use pretty_assertions::assert_eq;
    use rusoto_cloudwatch::{
        Datapoint,
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
    fn test_get_metric_statistics() {
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

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::get_metric_statistics(&client, &bucket))
            .unwrap();

        let datapoints = vec![
            Datapoint {
                average:   Some(123456789.0),
                timestamp: Some("2020-03-01T20:59:00Z".into()),
                unit:      Some("Bytes".into()),
                ..Default::default()
            },
        ];

        let expected = vec![
            GetMetricStatisticsOutput {
                datapoints: Some(datapoints),
                label:      Some("BucketSizeBytes".into()),
            },
        ];

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_iso8601() {
        let dt       = Utc.ymd(2020, 3, 1).and_hms(0, 16, 27);
        let expected = "2020-03-01T00:16:27Z";

        let client = mock_client(None);
        let ret    = Client::iso8601(&client, dt);

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_list_metrics() {
        let mut client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
        );

        let ret = Runtime::new()
            .unwrap()
            .block_on(Client::list_metrics(&mut client))
            .unwrap();

        let expected = vec![
            Metric {
                metric_name: Some("BucketSizeBytes".into()),
                namespace:   Some("AWS/S3".into()),
                dimensions:  Some(vec![
                    Dimension {
                        name:  "BucketName".into(),
                        value: "a-bucket-name".into(),
                    },
                    Dimension {
                        name:  "StorageType".into(),
                        value: "StandardStorage".into(),
                    },
                ]),
            },
            Metric {
                metric_name: Some("BucketSizeBytes".into()),
                namespace:   Some("AWS/S3".into()),
                dimensions:  Some(vec![
                    Dimension {
                        name:  "BucketName".into(),
                        value: "a-bucket-name".into(),
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
                        name: "BucketName".into(),
                        value: "another-bucket-name".into(),
                    },
                    Dimension {
                        name: "StorageType".into(),
                        value: "StandardStorage".into(),
                    },
                ]),
            },
        ];

        assert_eq!(ret, expected);
    }
}
