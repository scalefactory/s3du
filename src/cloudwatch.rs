// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    anyhow,
    Result,
};
use chrono::prelude::*;
use chrono::Duration;
use log::debug;
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
use super::{
    BucketNames,
    BucketSizer,
};

const S3_BUCKETSIZEBYTES: &str = "BucketSizeBytes";
const S3_NAMESPACE: &str = "AWS/S3";

type StorageTypes = Vec<String>;

// This Hash is keyed by bucket name and contains a list of storage types that
// are used within the bucket.
#[derive(Debug, PartialEq)]
struct BucketMetrics(HashMap<String, StorageTypes>);

impl BucketMetrics {
    // Return the bucket names from the BucketMetrics
    fn bucket_names(&self) -> BucketNames {
        debug!(
            "BucketMetrics::bucket_names: Returning names from: {:#?}",
            self.0,
        );

        self.0.iter().map(|(k, _v)| k.to_string()).collect()
    }

    // Return storage types of a given bucket
    fn storage_types(&self, bucket: &str) -> &StorageTypes {
        self.0.get(bucket).unwrap()
    }
}

// Conversion from a Vec<Metric> as returned by AWS to our BucketMetrics
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

// A RefCell is used to keep the external API immutable while we can change
// metrics internally.
pub struct Client {
    client:  CloudWatchClient,
    metrics: Option<BucketMetrics>,
}

impl BucketSizer for Client {
    // Return a list of S3 bucket names from CloudWatch.
    fn list_buckets(&mut self) -> Result<BucketNames> {
        let metrics: BucketMetrics = self.list_metrics()?.into();
        let bucket_names           = metrics.bucket_names();

        self.metrics = Some(metrics);

        Ok(bucket_names)
    }

    // Get the size of a given bucket
    fn bucket_size(&self, bucket: &str) -> Result<usize> {
        debug!("bucket_size: Calculating size for '{}'", bucket);

        let mut size: usize = 0;

        // We need to know which storage types are available for a bucket.
        let metrics = match &self.metrics {
            Some(m) => m,
            None    => return Err(anyhow!("No bucket metrics")),
        };
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
            let input = GetMetricStatisticsInput {
                dimensions:  Some(dimensions),
                end_time:    self.iso8601(now),
                metric_name: S3_BUCKETSIZEBYTES.into(),
                namespace:   S3_NAMESPACE.into(),
                period:      one_day.num_seconds(),
                start_time:  self.iso8601(now - one_day),
                statistics:  Some(vec!["Average".into()]),
                unit:        Some("Bytes".into()),
                ..Default::default()
            };

            input
        })
        .collect();

        // Perform a query for each bucket storage type
        for input in inputs {
            debug!(
                "bucket_size: Performing API call for input: {:#?}",
                input,
            );

            let output = self.client.get_metric_statistics(input).sync()?;

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
            size = size + (bytes as usize);
        }

        debug!(
            "bucket_size: Calculated bucket size for '{}' is '{}'",
            bucket,
            size,
        );

        Ok(size)
    }
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
            let output = self.client.list_metrics(list_metrics_input)
                .sync()?;
                //.context("Failed to list metrics")?;

            debug!("list_metrics: API returned: {:#?}", output);

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

        debug!("list_metrics: Metrics collection: {:#?}", metrics);

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

    // Possibly helpful while debugging tests.
    fn init() {
        // Try init because we can only init the logger once.
        let _ = pretty_env_logger::try_init();
    }

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
    fn test_bucket_metrics_from() {
        init();

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
        init();

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
        init();

        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
            None,
        );
        let mut ret = Client::list_buckets(&mut client).unwrap();
        ret.sort();

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_iso8601() {
        let dt = Utc.ymd(2020, 3, 1).and_hms(0, 16, 27);
        let expected = "2020-03-01T00:16:27Z";

        let client = mock_client(None, None);
        let ret = Client::iso8601(&client, dt);

        assert_eq!(ret, expected);
    }

    #[test]
    fn test_bucket_size() {
        init();

        let metrics = get_metrics();
        let metrics: BucketMetrics = metrics.into();

        let client = mock_client(
            Some("cloudwatch-get-metric-statistics.xml"),
            Some(metrics),
        );

        let bucket = "some-other-bucket-name";
        let ret = Client::bucket_size(&client, bucket).unwrap();

        let expected = 123456789;

        assert_eq!(ret, expected);
    }
}
