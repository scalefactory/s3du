//! s3du: A tool for informing you of the used space in AWS S3 buckets.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::redundant_field_names)]
use anyhow::Result;
use clap::value_t;
use humansize::FileSize;
use log::{
    debug,
    info,
};
use rusoto_core::Region;
use std::str::FromStr;
use tokio::runtime::Runtime;

/// Command line parsing.
mod cli;

/// Common types and traits.
mod common;
use common::{
    BucketSizer,
    ClientConfig,
    ClientMode,
    SizeUnit,
};

#[cfg(feature = "s3")]
use common::ObjectVersions;

/// CloudWatch Client.
#[cfg(feature = "cloudwatch")]
mod cloudwatch;

/// S3 Client.
#[cfg(feature = "s3")]
mod s3;

/// Return the appropriate AWS client with the given `ClientConfig`.
fn client(config: ClientConfig) -> Box<dyn BucketSizer> {
    let mode   = &config.mode;
    let region = &config.region;

    info!("Fetching client in region {} for mode {:?}", region.name(), mode);

    match mode {
        #[cfg(feature = "cloudwatch")]
        ClientMode::CloudWatch => {
            let client = cloudwatch::Client::new(config);
            Box::new(client)
        },
        #[cfg(feature = "s3")]
        ClientMode::S3 => {
            let client = s3::Client::new(config);
            Box::new(client)
        },
    }
}

/// Return `size` as a human friendly size if requested by `SizeUnit`.
fn humansize(size: usize, unit: &SizeUnit) -> String {
    debug!("humansize: size {}, unit {:?}", size, unit);

    match unit {
        SizeUnit::Binary(unit)  => size.file_size(unit).unwrap(),
        SizeUnit::Bytes         => size.to_string(),
        SizeUnit::Decimal(unit) => size.file_size(unit).unwrap(),
    }
}

/// Perform the actual get and output of the bucket sizes.
async fn du(mut client: Box<dyn BucketSizer>, unit: SizeUnit) -> Result<()> {
    // List all of our buckets
    let buckets = client.buckets().await?;

    debug!("du: Got buckets: {:?}", buckets);

    // Track total size of all buckets.
    let mut total_size: usize = 0;

    // For each bucket name, get the size
    for bucket in buckets {
        let size = client.bucket_size(&bucket).await?;

        total_size += size;

        let size = humansize(size, &unit);

        println!("{size}\t{bucket}", size=size, bucket=bucket.name);
    }

    let total_size = humansize(total_size, &unit);

    // Display the total size the same way du(1) would, the total size followed
    // by a `.`.
    println!("{size}\t.", size=total_size);

    Ok(())
}

/// Entry point
fn main() -> Result<()> {
    pretty_env_logger::init();

    // Parse the CLI
    let matches = cli::parse_args();

    // Get the bucket name, if any.
    let bucket_name = match matches.value_of("BUCKET") {
        Some(name) => Some(name.to_string()),
        None       => None,
    };

    // Get the client mode
    let mode = value_t!(matches, "MODE", ClientMode)?;

    // Get the unit size to display
    let unit = value_t!(matches, "UNIT", SizeUnit)?;

    // Get the AWS_REGION
    // Safe to unwrap here as we validated the argument while parsing the CLI.
    let region = matches.value_of("REGION").unwrap();
    let region = Region::from_str(region)?;

    let mut config = ClientConfig {
        bucket_name: bucket_name,
        mode:        mode,
        region:      region,
        ..Default::default()
    };

    // If have s3 mode available we also need to pull in the ObjectVersions
    // from the command line.
    #[cfg(feature = "s3")]
    {
        if config.mode == ClientMode::S3 {
            // This should be safe, we validated this in the CLI parser.
            let versions = matches.value_of("OBJECT_VERSIONS").unwrap();

            // This should be safe, due to validation of the above.
            let versions = ObjectVersions::from_str(versions).unwrap();

            config.object_versions = versions;
        }
    }

    // The region here will come from CLI args in the future
    let client = client(config);

    Runtime::new()?.block_on(du(client, unit))
}
