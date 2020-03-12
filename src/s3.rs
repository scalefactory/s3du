// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::{
    Result,
};
use log::debug;
use rusoto_core::Region;
use rusoto_s3::{
    ListBucketsOutput,
    ListObjectsV2Request,
    Object,
    S3,
    S3Client,
};
use super::common::{
    BucketNames,
    BucketSizer,
};

struct BucketList(Vec<String>);

impl From<ListBucketsOutput> for BucketList {
    fn from(output: ListBucketsOutput) -> Self {
        let mut bucket_names = vec![];

        let buckets = match output.buckets {
            Some(buckets) => buckets,
            None          => vec![],
        };

        for bucket in buckets {
            if let Some(name) = bucket.name {
                bucket_names.push(name);
            }
        }

        BucketList(bucket_names)
    }
}

impl BucketList {
    fn bucket_names(&self) -> &BucketNames {
        &self.0
    }
}

// A RefCell is used to keep the external API immutable while we can change
// metrics internally.
pub struct Client {
    client:  S3Client,
    buckets: Option<BucketList>,
}

impl BucketSizer for Client {
    // Return a list of S3 bucket names from CloudWatch.
    fn list_buckets(&mut self) -> Result<BucketNames> {
        let bucket_list: BucketList = self.client.list_buckets().sync()?.into();
        let bucket_names            = bucket_list.bucket_names().to_owned();

        self.buckets = Some(bucket_list);

        Ok(bucket_names)
    }

    // Get the size of a given bucket
    fn bucket_size(&self, bucket: &str) -> Result<usize> {
        debug!("bucket_size: Calculating size for '{}'", bucket);

        let mut size: usize = 0;

        let objects = self.list_objects(bucket)?;

        for object in objects {
            if let Some(s) = object.size {
                size += s as usize;
            }
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
            "new: Creating S3Client in region '{}'",
            region.name(),
        );

        let client = S3Client::new(region);

        Client {
            client:  client,
            buckets: None,
        }
    }

    // This is currently bad, the objects vec could be huge
    fn list_objects(&self, bucket: &str) -> Result<Vec<Object>> {
        let mut continuation_token = None;
        let mut objects            = vec![];
        let mut start_after        = None;

        // Loop until all objects are processed.
        loop {
            let input = ListObjectsV2Request {
                bucket:             bucket.into(),
                continuation_token: continuation_token,
                start_after:        start_after.to_owned(),
                ..Default::default()
            };

            let output = self.client.list_objects_v2(input).sync()?;

            if let Some(contents) = output.contents {
                objects.extend(contents);
            }

            if let Some(sa) = output.start_after {
                start_after = Some(sa);
            }

            match output.continuation_token {
                Some(ct) => continuation_token = Some(ct),
                None     => break,
            };
        }

        Ok(objects)
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

    //// Create a mock CloudWatch client, returning the data from the specified
    //// data_file.
    fn mock_client(
        data_file: Option<&str>,
    ) -> Client {
        let data = match data_file {
            None    => "".to_string(),
            Some(d) => MockResponseReader::read_response("test-data", d.into()),
        };

        let client = S3Client::new_with(
            MockRequestDispatcher::default().with_body(&data),
            MockCredentialsProvider,
            Default::default()
        );

        Client {
            client:  client,
            buckets: None,
        }
    }

    //#[test]
    //fn test_bucket_metrics_from() {
    //    init();

    //    let metrics = get_metrics();

    //    // Get the above into our BucketMetrics
    //    let metrics: BucketMetrics = metrics.into();

    //    let mut expected = HashMap::new();
    //    expected.insert("some-bucket-name".into(), vec![
    //        "StandardStorage".into(),
    //        "StandardIAStorage".into(),
    //    ]);
    //    expected.insert("some-other-bucket-name".into(), vec![
    //        "StandardStorage".into(),
    //    ]);

    //    let expected = BucketMetrics(expected);

    //    assert_eq!(metrics, expected);
    //}

    //#[test]
    //fn test_bucket_metrics_bucket_names() {
    //    init();

    //    let metrics = get_metrics();

    //    // Get the above into our BucketMetrics
    //    let metrics: BucketMetrics = metrics.into();
    //    let mut ret = metrics.bucket_names();
    //    ret.sort();

    //    let expected = vec![
    //        "some-bucket-name",
    //        "some-other-bucket-name",
    //    ];

    //    assert_eq!(ret, expected);
    //}

    #[test]
    fn test_list_buckets() {
        init();

        let expected = vec![
            "a-bucket-name",
            "another-bucket-name",
        ];

        let mut client = mock_client(
            Some("s3-list-buckets.xml"),
        );
        let mut ret = Client::list_buckets(&mut client).unwrap();
        ret.sort();

        assert_eq!(ret, expected);
    }

    //#[test]
    //fn test_bucket_size() {
    //    init();

    //    let metrics = get_metrics();
    //    let metrics: BucketMetrics = metrics.into();

    //    let client = mock_client(
    //        Some("cloudwatch-get-metric-statistics.xml"),
    //        Some(metrics),
    //    );

    //    let bucket = "some-other-bucket-name";
    //    let ret = Client::bucket_size(&client, bucket).unwrap();

    //    let expected = 123456789;

    //    assert_eq!(ret, expected);
    //}
}
