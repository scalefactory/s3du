// s3du: A tool for informing you of the used space in AWS S3.
use anyhow::{
    Context,
    Result,
};
use rusoto_core::Region;

const DEFAULT_REGION: Region = Region::EuWest1;

mod cloudwatch;

fn main() -> Result<()> {
    let cloudwatch_client = cloudwatch::Client::new(DEFAULT_REGION);

    let bucket_names = cloudwatch_client.list_buckets()
        .context("Failed to list buckets")?;

    println!("{:?}", bucket_names);

    //let metrics = cloudwatch_client.list_metrics(list_metrics_input)
    //    .sync()
    //    .context("Failed to list metrics")?;

    //println!("{:?}", metrics);

    Ok(())
}
