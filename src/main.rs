// s3du: A tool for informing you of the used space in AWS S3.
use anyhow::Result;
use humansize::{
    file_size_opts,
    FileSize,
};
use log::{
    debug,
    info,
};
use rusoto_core::Region;

const DEFAULT_REGION: Region = Region::EuWest1;

// These are used by the CloudWatch and S3 modes.
type BucketNames = Vec<String>;
trait BucketSizer {
    fn list_buckets(&mut self) -> Result<BucketNames>;
    fn bucket_size(&self, bucket: &str) -> Result<usize>;
}

mod cloudwatch;

#[derive(Debug)]
enum ClientMode {
    CloudWatch,
    S3,
}

// Return the appropriate AWS client for fetching the bucket size
fn client(mode: ClientMode, region: Region) -> impl BucketSizer {
    info!("Fetching client in region {} for mode {:?}", region.name(), mode);

    match mode {
        ClientMode::CloudWatch => cloudwatch::Client::new(region),
        ClientMode::S3         => unimplemented!(),
    }
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    let mode = ClientMode::CloudWatch;

    let mut client = client(mode, DEFAULT_REGION);

    let bucket_names = client.list_buckets()?;

    println!("{:?}", bucket_names);

    for bucket in bucket_names {
        let size = client.bucket_size(&bucket)?;
        let size = size.file_size(file_size_opts::BINARY).unwrap();
        println!("{}: {}", bucket, size);
    }

    Ok(())
}
