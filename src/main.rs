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

    // This will come from CLI args in the future
    let mode = ClientMode::CloudWatch;

    // The region here will come from CLI args in the future
    let mut client = client(mode, DEFAULT_REGION);

    // List all of our buckets
    let bucket_names = client.list_buckets()?;

    debug!("main: Got bucket names: {:?}", bucket_names);

    // For each bucket name, get the size
    for bucket in bucket_names {
        let size = client.bucket_size(&bucket)?;

        // If the above didn't error, it should always be safe to unwrap the
        // usize here.
        let size = size.file_size(file_size_opts::BINARY).unwrap();

        println!("{}: {}", bucket, size);
    }

    Ok(())
}
