//! s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use clap::value_t;
use humansize::{
    file_size_opts,
    FileSize,
};
use log::{
    debug,
    info,
};
use rusoto_core::Region;
use std::str::FromStr;

mod cli;
mod cloudwatch;

// Valid modes that s3du can operate in.
#[derive(Debug)]
enum ClientMode {
    CloudWatch,
    S3,
}

// These are used by the CloudWatch and S3 modes.
type BucketNames = Vec<String>;
trait BucketSizer {
    fn list_buckets(&mut self) -> Result<BucketNames>;
    fn bucket_size(&self, bucket: &str) -> Result<usize>;
}

// This is used to work out which mode we're in after parsing the CLI.
// We shouldn't ever hit the error condition here.
impl FromStr for ClientMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cloudwatch" => Ok(Self::CloudWatch),
            "s3"         => Ok(Self::S3),
            _            => Err("no match"),
        }
    }
}

// Return the appropriate AWS client for fetching the bucket size
fn client(mode: ClientMode, region: Region) -> impl BucketSizer {
    info!("Fetching client in region {} for mode {:?}", region.name(), mode);

    match mode {
        ClientMode::CloudWatch => cloudwatch::Client::new(region),
        ClientMode::S3         => unimplemented!(),
    }
}

// du: Perform the actual get and output of the bucket sizes.
fn du(mut client: impl BucketSizer) -> Result<()> {
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

// Entry point
fn main() -> Result<()> {
    pretty_env_logger::init();

    // Parse the CLI
    let matches = cli::parse_args();

    // This will come from CLI args in the future
    // Get the client mode
    let mode = value_t!(matches, "MODE", ClientMode)?;

    // Get the AWS_REGION
    // Safe to unwrap here as we validated the argument while parsing the CLI.
    let region = matches.value_of("REGION").unwrap();
    let region = Region::from_str(region)?;

    // The region here will come from CLI args in the future
    let client = client(mode, region);

    du(client)
}
