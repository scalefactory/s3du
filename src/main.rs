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
use tokio::runtime::Runtime;

mod cli;
mod common;
use common::{
    BucketSizer,
    ClientConfig,
    ClientMode,
    SizeUnit,
};

#[cfg(feature = "s3")]
use common::S3ObjectVersions;

#[cfg(feature = "cloudwatch")]
mod cloudwatch;
#[cfg(feature = "s3")]
mod s3;

// Return the appropriate AWS client for fetching the bucket size
fn client(config: ClientConfig) -> Box<dyn BucketSizer> {
    let mode   = &config.mode;
    let region = &config.region;

    info!("Fetching client in region {} for mode {:?}", region.name(), mode);

    match mode {
        #[cfg(feature = "cloudwatch")]
        ClientMode::CloudWatch => Box::new(cloudwatch::Client::new(config.region)),
        #[cfg(feature = "s3")]
        ClientMode::S3         => Box::new(s3::Client::new(config)),
    }
}

// Return a filesize as a human readable size, if that was requested
fn humansize(size: usize, unit: &SizeUnit) -> String {
    match unit {
        SizeUnit::Binary  => size.file_size(file_size_opts::BINARY).unwrap(),
        SizeUnit::Bytes   => size.to_string(),
        SizeUnit::Decimal => size.file_size(file_size_opts::DECIMAL).unwrap(),
    }
}

// du: Perform the actual get and output of the bucket sizes.
async fn du(mut client: Box<dyn BucketSizer>, unit: SizeUnit) -> Result<()> {
    // List all of our buckets
    let bucket_names = client.list_buckets().await?;

    debug!("main: Got bucket names: {:?}", bucket_names);

    // For each bucket name, get the size
    for bucket in bucket_names {
        let size = client.bucket_size(&bucket).await?;
        let size = humansize(size, &unit);

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

    // Get the unit size to display
    let unit = value_t!(matches, "UNIT", SizeUnit)?;

    // Get the AWS_REGION
    // Safe to unwrap here as we validated the argument while parsing the CLI.
    let region = matches.value_of("REGION").unwrap();
    let region = Region::from_str(region)?;

    let mut config = ClientConfig {
        mode:   mode,
        region: region,
        ..Default::default()
    };

    #[cfg(feature = "s3")]
    {
        if config.mode == ClientMode::S3 {
            let versions = matches.value_of("OBJECT_VERSIONS").unwrap();
            let versions = S3ObjectVersions::from_str(versions).unwrap();

            config.s3_object_versions = versions;
        }
    }

    // The region here will come from CLI args in the future
    let client = client(config);

    Runtime::new()?.block_on(du(client, unit))
}
