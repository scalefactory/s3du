// Handles the CloudWatch bucket metrics
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use crate::common::{
    BucketNames,
    StorageTypes,
};
use log::debug;
use aws_sdk_cloudwatch::model::Metric;
use std::collections::HashMap;

// This Hash is keyed by bucket name and contains a list of storage types that
// are used within the bucket.
/// Holds a HashMap of bucket names and their storage types.
#[derive(Debug, Eq, PartialEq)]
pub struct BucketMetrics(pub HashMap<String, StorageTypes>);

impl BucketMetrics {
    /// Return the bucket names from the `BucketMetrics`.
    pub fn bucket_names(&self) -> BucketNames {
        debug!(
            "BucketMetrics::bucket_names: Returning names from: {:#?}",
            self.0,
        );

        self.0
            .keys()
            .map(|k| k.to_string())
            .collect()
    }

    /// Return storage types of a given bucket.
    pub fn storage_types(&self, bucket: &str) -> &StorageTypes {
        // Unwrap should be safe here, elsewhere we already check that the
        // bucket is valid.
        self.0
            .get(bucket)
            .unwrap()
    }
}

/// Conversion from a `Vec<Metric>` as returned by AWS to our `BucketMetrics`.
impl From<Vec<Metric>> for BucketMetrics {
    fn from(metrics: Vec<Metric>) -> Self {
        debug!("From: Vec<Metric> for BucketMetrics");

        let mut bucket_metrics = HashMap::new();

        for metric in metrics {
            // Get the dimensions if any, otherwise skip to next iteration
            let dimensions = match metric.dimensions {
                Some(d) => d,
                None    => continue,
            };

            // Storage for what we'll pull out of the dimensions
            let mut name          = String::new();
            let mut storage_type  = String::new();

            // Process the dimensions, taking the bucket name and storage types
            for dimension in dimensions {
                // Extract the dimension name
                let dname = match dimension.name {
                    Some(n) => n,
                    None    => continue,
                };

                match dname.as_ref() {
                    "BucketName"  => name         = dimension.value.unwrap(),
                    "StorageType" => storage_type = dimension.value.unwrap(),
                    _             => {},
                }
            }

            // Get the existing StorageTypes entry for the bucket, or create a
            // new one if it doesn't exist yet.
            let storage_types = bucket_metrics
                .entry(name)
                .or_insert_with(StorageTypes::new);

            // Push the new storage type into the vec
            storage_types.push(storage_type);
        }

        BucketMetrics(bucket_metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatch::model::{
        Dimension,
        Metric,
    };
    use pretty_assertions::assert_eq;

    // Metrics used in the tests
    fn get_metrics() -> Vec<Metric> {
        vec![
            Metric::builder()
                .metric_name("BucketSizeBytes")
                .namespace("AWS/S3")
                .set_dimensions(Some(vec![
                    Dimension::builder()
                        .name("BucketName")
                        .value("some-bucket-name")
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
                        .value("some-bucket-name")
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
                        .value("some-other-bucket-name")
                        .build(),

                    Dimension::builder()
                        .name("StorageType")
                        .value("StandardStorage")
                        .build(),
                ]))
                .build(),
        ]
    }

    #[test]
    fn test_bucket_metrics_from() {
        let metrics = get_metrics();

        // Get the above into our BucketMetrics
        let metrics: BucketMetrics = metrics.into();

        let mut expected = HashMap::new();
        expected.insert("some-bucket-name".into(), vec![
            "StandardIAStorage".into(),
            "StandardStorage".into(),
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
}
