//! s3du: A tool for informing you of the used space in AWS S3 buckets.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::redundant_field_names)]
use anyhow::Result;
use clap::value_t;
use log::{
    debug,
    info,
};
use std::str::FromStr;

/// Command line parsing.
mod cli;

/// Common types and traits.
mod common;
use common::{
    BucketSizer,
    ClientConfig,
    ClientMode,
    HumanSize,
    Region,
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

/// `Client` struct wraps a `Box<dyn BucketSizer>`.
struct Client(Box<dyn BucketSizer>);

/// `Client` implementation.
impl Client {
    /// Return the appropriate AWS client with the given `ClientConfig`.
    async fn new(config: ClientConfig) -> Self {
        let mode   = &config.mode;
        let region = &config.region;

        info!("Client in region {} for mode {:?}", region.name(), mode);

        let client: Box<dyn BucketSizer> = match mode {
            #[cfg(feature = "cloudwatch")]
            ClientMode::CloudWatch => {
                let client = cloudwatch::Client::new(config);
                Box::new(client.await)
            },
            #[cfg(feature = "s3")]
            ClientMode::S3 => {
                let client = s3::Client::new(config);
                Box::new(client.await)
            },
        };

        Client(client)
    }

    /// Perform the actual get and output of the bucket sizes.
    async fn du(&self, unit: SizeUnit) -> Result<()> {
        // List all of our buckets
        let buckets = self.0.buckets().await?;

        debug!("du: Got buckets: {:?}", buckets);

        // Track total size of all buckets.
        let mut total_size: usize = 0;

        // For each bucket name, get the size
        for bucket in buckets {
            let size = self.0.bucket_size(&bucket).await?;

            total_size += size;

            let size = size.humansize(&unit);

            println!("{size}\t{bucket}", size=size, bucket=bucket.name);
        }

        let total_size = total_size.humansize(&unit);

        // Display the total size the same way du(1) would, the total size
        // followed by a `.`.
        println!("{size}\t.", size=total_size);

        Ok(())
    }
}

/// Entry point
#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Parse the CLI
    let matches = cli::parse_args();

    // Get the bucket name, if any.
    let bucket_name = matches
        .value_of("BUCKET")
        .map(|name| name.to_string());

    // Get the client mode
    let mode = value_t!(matches, "MODE", ClientMode)?;

    // Get the unit size to display
    let unit = value_t!(matches, "UNIT", SizeUnit)?;

    // Here we get the region, if a custom endpoint is set, that is used,
    // otherwise we get the regular region.
    // Unwraps on values here should be fine, as they're checked when the CLI
    // is validated.
    #[cfg(feature = "s3")]
    let region = if matches.is_present("ENDPOINT") {
        if mode == ClientMode::S3 {
            let endpoint = matches.value_of("ENDPOINT").unwrap();

            Region::new().await.set_endpoint(endpoint)
        }
        else {
            eprintln!("Error: Endpoint supplied but client mode is not S3");
            ::std::process::exit(1);
        }
    }
    else {
        let region = matches.value_of("REGION").unwrap();
        Region::new().await.set_region(region)
    };

    // Endpoint selection isn't supported for CloudWatch, so we can drop it if
    // we're compiled without the S3 feature.
    #[cfg(all(feature = "cloudwatch", not(feature = "s3")))]
    let region = {
        let region = matches.value_of("REGION").unwrap();
        Region::new().await.set_region(region)
    };

    // This warning will trigger if compiled without the "s3" feature. We're
    // aware, allow it.
    #[allow(unused_mut)]
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
    let client = Client::new(config).await;

    client.du(unit).await
}
