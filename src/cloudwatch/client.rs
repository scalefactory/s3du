// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use chrono::prelude::*;
use log::debug;
use rusoto_core::Region;
use rusoto_cloudwatch::{
    CloudWatch,
    CloudWatchClient,
    ListMetricsInput,
    Metric,
};
use super::bucket_metrics::BucketMetrics;

const S3_BUCKETSIZEBYTES: &str = "BucketSizeBytes";
const S3_NAMESPACE: &str = "AWS/S3";

pub struct Client {
    pub client:  CloudWatchClient,
    pub metrics: Option<BucketMetrics>,
}

impl Client {
    // Return a new CloudWatchClient in the specified region.
    pub fn new(region: Region) -> Self {
        debug!(
            "new: Creating CloudWatchClient in region '{}'",
            region.name(),
        );

        let client = CloudWatchClient::new(region);

        Client {
            client:  client,
            metrics: None,
        }
    }

    // Return an ISO8601 formatted timestamp suitable for
    // GetMetricsStatisticsInput.
    pub fn iso8601(&self, dt: DateTime<Utc>) -> String {
        dt.to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    // Get list of buckets with BucketSizeBytes metrics.
    // An individual metric resembles the following:
    // Metric {
    //   dimensions: Some([
    //     Dimension {
    //       name: "StorageType",
    //       value: "StandardStorage"
    //     },
    //     Dimension {
    //       name: "BucketName",
    //       value: "some-bucket-name"
    //     }
    //   ]),
    //   metric_name: Some("BucketSizeBytes"),
    //   namespace: Some("AWS/S3")
    // }
    pub async fn list_metrics(&self) -> Result<Vec<Metric>> {
        debug!("list_metrics: Listing...");

        let metric_name    = S3_BUCKETSIZEBYTES.to_string();
        let namespace      = S3_NAMESPACE.to_string();
        let mut metrics    = vec![];
        let mut next_token = None;

        // We loop until we've processed everything.
        loop {
            // Input for CloudWatch API
            let list_metrics_input = ListMetricsInput {
                metric_name: Some(metric_name.clone()),
                namespace:   Some(namespace.clone()),
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

    #[test]
    fn test_iso8601() {
        let dt       = Utc.ymd(2020, 3, 1).and_hms(0, 16, 27);
        let expected = "2020-03-01T00:16:27Z";

        let client = mock_client(None, None);
        let ret    = Client::iso8601(&client, dt);

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_list_metrics() {
        let mut client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
            None,
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
