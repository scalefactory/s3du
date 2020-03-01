// s3du: A tool for informing you of the used space in AWS S3.
use anyhow::{
    Context,
    Result,
};
use humansize::{
    file_size_opts,
    FileSize,
};
use rusoto_core::Region;

const DEFAULT_REGION: Region = Region::EuWest1;

mod cloudwatch;

fn main() -> Result<()> {
    let mut cloudwatch_client = cloudwatch::Client::new(DEFAULT_REGION);

    let bucket_names = cloudwatch_client.list_buckets()
        .context("Failed to list buckets")?;

    println!("{:?}", bucket_names);

    for bucket in bucket_names {
        let size = cloudwatch_client.bucket_size(&bucket)?;
        let size = size.file_size(file_size_opts::BINARY).unwrap();
        println!("{}: {}", bucket, size);
    }

    //let metrics = cloudwatch_client.list_metrics(list_metrics_input)
    //    .sync()
    //    .context("Failed to list metrics")?;

    //println!("{:?}", metrics);

    Ok(())
}
