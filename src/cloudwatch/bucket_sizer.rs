// Implements the BucketSizer trait for CloudWatch Client
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    anyhow,
    Result,
};
use async_trait::async_trait;
use crate::common::{
    Bucket,
    Buckets,
    BucketSizer,
};
use super::bucket_metrics::BucketMetrics;
use super::client::Client;
use tracing::debug;

#[async_trait]
impl BucketSizer for Client {
    /// Return a list of S3 bucket names from CloudWatch.
    /// We also cache the returned metrics here, since we need to reference this
    /// elsewhere, and we don't want to have to query for it again.
    async fn buckets(&self) -> Result<Buckets> {
        debug!("buckets: Listing...");

        let metrics: BucketMetrics = self.list_metrics().await?.into();

        let mut buckets = Buckets::new();

        for bucket in metrics.bucket_names() {
            let storage_types = metrics.storage_types(&bucket).clone();

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
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    async fn bucket_size(&self, bucket: &Bucket) -> Result<u64> {
        let bucket_name = &bucket.name;

        debug!("bucket_size: Calculating size for '{}'", bucket_name);

        let mut size = 0;

        let metric_statistics = self.get_metric_statistics(bucket).await?;
        for stats in metric_statistics {
            // If we don't get any datapoints, proceed to the next input.
            let Some(mut datapoints) = stats.datapoints else {
                continue
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
                b.timestamp.cmp(&a.timestamp)
            });

            let datapoint = &datapoints[0];

            // BucketSizeBytes only supports Average, so this should be safe
            // to unwrap.
            let bytes = datapoint.average
                .expect("Couldn't unwrap average");

            // Add up the size of each storage type
            // Do a bit of rounding here to get an integer value before
            // converting to u64.
            size += bytes.round() as u64;
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
    use aws_sdk_cloudwatch::{
        client::Client as CloudWatchClient,
        config::Config as CloudWatchConfig,
        config::Credentials,
    };
    use aws_smithy_runtime::client::http::test_util::{
        ReplayEvent,
        StaticReplayClient,
    };
    use aws_smithy_types::body::SdkBody;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;

    // Create a mock CloudWatch client, returning the data from the specified
    // data_file.
    fn mock_client(
        data_file: Option<&str>,
    ) -> Client {
        let data = match data_file {
            None    => "".to_string(),
            Some(d) => {
                let path = Path::new("test-data").join(d);
                fs::read_to_string(path).unwrap()
            },
        };

        let http_client = StaticReplayClient::new(vec![
            ReplayEvent::new(
                http::Request::builder()
                    .body(SdkBody::from("request body"))
                    .unwrap(),

                http::Response::builder()
                    .status(200)
                    .body(SdkBody::from(data))
                    .unwrap(),
            ),
        ]);

        let creds = Credentials::from_keys(
            "ATESTCLIENT",
            "atestsecretkey",
            Some("atestsecrettoken".to_string()),
        );

        let conf = CloudWatchConfig::builder()
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

    #[tokio::test]
    async fn test_buckets() {
        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let client = mock_client(
            Some("cloudwatch-list-metrics.xml"),
        );

        let buckets = client.buckets().await.unwrap();

        let mut buckets: Vec<String> = buckets.iter()
            .map(|b| b.name.to_owned())
            .collect();

        buckets.sort();

        assert_eq!(buckets, expected);
    }

    #[tokio::test]
    async fn test_bucket_size() {
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

        let ret = client.bucket_size(&bucket).await.unwrap();

        let expected = 123_456_789;

        assert_eq!(ret, expected);
    }
}
