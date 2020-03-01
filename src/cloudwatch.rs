// s3du: A tool for informing you of the used space in AWS S3.
use anyhow::{
    Context,
    Result,
};
use chrono::prelude::*;
use chrono::Duration;
use rusoto_core::Region;
use rusoto_cloudwatch::{
    CloudWatch,
    CloudWatchClient,
    Dimension,
    GetMetricStatisticsInput,
    ListMetricsInput,
    Metric,
};
use std::collections::HashMap;

const S3_BUCKET_SIZE_BYTES: &str = "BucketSizeBytes";
const S3_NAMESPACE: &str = "AWS/S3";

type BucketNames = Vec<String>;
type StorageTypes = Vec<String>;

// This Hash is keyed by bucket name and contains a list of storage types that
// are used within the bucket.
#[derive(Debug, PartialEq)]
struct BucketMetrics(HashMap<String, StorageTypes>);

impl BucketMetrics {
    // Return the bucket names from the BucketMetrics
    fn bucket_names(&self) -> BucketNames {
        self.0.iter().map(|(k, _v)| k.to_string()).collect()
    }
}

impl From<Vec<Metric>> for BucketMetrics {
    fn from(metrics: Vec<Metric>) -> Self {
        let mut bucket_metrics = HashMap::new();

        for metric in metrics {
            // Get the dimensions if any, otherwise skip to next iteration
            let dimensions = match metric.dimensions {
                Some(d) => d,
                None    => continue,
            };

            // Storage for what we'll pull out of the dimensions
            let mut name = String::new();
            let mut storage_types = vec![];

            // Process the dimensions, taking the bucket name and storage types
            for dimension in dimensions {
                match dimension.name.as_ref() {
                    "BucketName"  => name = dimension.value,
                    "StorageType" => storage_types.push(dimension.value),
                    _             => {},
                }
            }

            // Set the storage types for this bucket
            bucket_metrics.insert(name, storage_types);
        }

        BucketMetrics(bucket_metrics)
    }
}

pub struct Client {
    client: CloudWatchClient,
}

impl Client {
    // Return a new CloudWatchClient in the specified region.
    pub fn new(region: Region) -> Self {
        let client = CloudWatchClient::new(region);

        Client {
            client: client,
        }
    }

    // Return a list of S3 bucket names from CloudWatch.
    pub fn list_buckets(&self) -> Result<BucketNames> {
        let metrics: BucketMetrics = self.list_metrics()?.into();
        let bucket_names           = metrics.bucket_names();

        Ok(bucket_names)
    }

    // Get the size of a given bucket
    pub fn bucket_size(&self, bucket: &str) -> Result<u64> {
        let mut size: u64 = 0;

        // Get the time now so we can select the last 24 hours of metrics.
        let now: DateTime<Utc> = Utc::now();
        let one_day = Duration::days(1);

        // Dimensions for bucket selection
        let dimensions = vec![
            Dimension {
                name:  "BucketName".into(),
                value: bucket.into(),
            },
            Dimension {
                name:  "StorageType".into(),
                value: "StandardStorage".into(),
            },
        ];

        let input = GetMetricStatisticsInput {
            dimensions:  Some(dimensions),
            end_time:    self.iso8601(now - one_day),
            metric_name: S3_BUCKET_SIZE_BYTES.into(),
            namespace:   S3_NAMESPACE.into(),
            period:      one_day.num_seconds(),
            start_time:  self.iso8601(now),
            ..Default::default()
        };

        Ok(size)
    }

    // Return an ISO8601 formatted timestamp suitable for
    // GetMetricsStatisticsInput.
    fn iso8601(&self, dt: DateTime<Utc>) -> String {
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
    fn list_metrics(&self) -> Result<Vec<Metric>> {
        let metric_name    = S3_BUCKET_SIZE_BYTES.to_string();
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
            let output = self.client.list_metrics(list_metrics_input)
                .sync()?;
                //.context("Failed to list metrics")?;

            // If we get any metrics, append them to our vec
            match output.metrics {
                Some(m) => metrics.append(&mut m.clone()),
                None    => {},
            };

            // If there was a next token, use it, otherwise the loop is done.
            match output.next_token {
                Some(t) => next_token = Some(t),
                None    => break,
            };
        }

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rusoto_mock::{
        MockCredentialsProvider,
        MockRequestDispatcher,
        MockResponseReader,
        ReadMockResponse,
    };

    // Create a mock CloudWatch client, returning the data from the specified
    // data_file.
    fn mock_client(data_file: Option<&str>) -> Client {
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
            client: client,
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
    fn test_bucket_metrics_from() {
        let metrics = get_metrics();

        // Get the above into our BucketMetrics
        let metrics: BucketMetrics = metrics.into();

        let mut expected = HashMap::new();
        expected.insert("some-bucket-name".into(), vec![
            "StandardStorage".into(),
            "StandardIAStorage".into(),
        ]);
        expected.insert("some-other-bucket-name".into(), vec![
            "StandardStorage".into(),
        ]);

        let expected = BucketMetrics(expected);

        assert_eq!(metrics, expected);
    }

    #[test]
    fn test_bucket_metrics_bucket_names() {
        let metrics = get_metrics();

        // Get the above into our BucketMetrics
        let metrics: BucketMetrics = metrics.into();
        let mut ret = metrics.bucket_names();
        ret.sort();

        let expected = vec![
            "some-bucket-name",
            "some-other-bucket-name",
        ];

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_list_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let client = mock_client(Some("cloudwatch-list-metrics.xml"));
        let mut ret = Client::list_buckets(&client).unwrap();
        ret.sort();

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_iso8601() {
        let dt = Utc.ymd(2020, 3, 1).and_hms(0, 16, 27);
        let expected = "2020-03-01T00:16:27Z";

        let client = mock_client(None);
        let ret = Client::iso8601(&client, dt);

        assert_eq!(ret, expected);
    }
}
