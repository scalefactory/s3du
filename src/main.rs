//! s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::redundant_field_names)]
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
mod common;
use common::{
    BucketSizer,
    ClientMode,
};
mod s3;

// Return the appropriate AWS client for fetching the bucket size
fn client(mode: ClientMode, region: Region) -> Box<dyn BucketSizer> {
    info!("Fetching client in region {} for mode {:?}", region.name(), mode);

    match mode {
        ClientMode::CloudWatch => Box::new(cloudwatch::Client::new(region)),
        ClientMode::S3         => Box::new(s3::Client::new(region)),
    }
}

// du: Perform the actual get and output of the bucket sizes.
fn du(mut client: Box<dyn BucketSizer>) -> Result<()> {
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
