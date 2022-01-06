// cli: This module is responsible for command line interface parsing
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
    App,
    Arg,
    ArgMatches,
};
use lazy_static::lazy_static;
use log::debug;
use std::env;

#[cfg(feature = "s3")]
use url::Url;

// Our fallback default region if we fail to find a region in the environment
const FALLBACK_REGION: &str = "us-east-1";

// This catches cases where we've compiled with either:
//   - Only "cloudwatch"
//   - Both "cloudwatch" and "s3"
/// Default mode that `s3du` runs in.
#[cfg(feature = "cloudwatch")]
const DEFAULT_MODE: &str = "cloudwatch";

// This catches cases where we've compiled with:
//   - Only "s3"
/// Default mode that `s3du` runs in.
#[cfg(all(feature = "s3", not(feature = "cloudwatch")))]
const DEFAULT_MODE: &str = "s3";

/// Default object versions to sum in S3 mode.
#[cfg(feature = "s3")]
const DEFAULT_OBJECT_VERSIONS: &str = "current";

lazy_static! {
    /// Default AWS region if one isn't provided on the command line.
    ///
    /// Obtains the default region in the following order:
    ///   - `AWS_DEFAULT_REGION` environment variable
    ///   - `AWS_REGION` environment variable
    ///   - Falls back to `us-east-1` if regions in the environment variables
    ///     are unavailable
    static ref DEFAULT_REGION: String = {
        // Attempt to find the default via AWS_REGION and AWS_DEFAULT_REGION
        // If we don't find a region, we'll fall back to our FALLBACK_REGION
        let possibilities = vec![
            env::var("AWS_REGION"),
            env::var("AWS_DEFAULT_REGION"),
        ];

        let region = possibilities
            .iter()
            .find_map(|region| region.as_ref().ok())
            .map_or_else(
                || FALLBACK_REGION,
                |r| r,
            );

        region.to_string()
    };
}

/// Default unit to display sizes in.
const DEFAULT_UNIT: &str = "binary";

// This should match the string values in the ClientMode FromStr impl in
// common.
/// Valid modes for the `--mode` command line switch.
const VALID_MODES: &[&str] = &[
    #[cfg(feature = "cloudwatch")]
    "cloudwatch",
    #[cfg(feature = "s3")]
    "s3",
];

// This should match the string values in the UnitSize FromStr impl in common.
/// Valid unit sizes for the `--unit` command line switch.
const VALID_SIZE_UNITS: &[&str] = &[
    "binary",
    "bytes",
    "decimal",
];

// This should match the ObjectVersions in the common.rs
/// Valid S3 object versions for the `--object-versions` switch.
#[cfg(feature = "s3")]
const OBJECT_VERSIONS: &[&str] = &[
    "all",
    "current",
    "multipart",
    "non-current",
];

/// Ensures that a given bucket name is valid.
///
/// This validation is taken from
/// https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html.
/// We validate based on the legacy standard for compatibility.
fn is_valid_aws_s3_bucket_name(s: &str) -> Result<(), String> {
    // Bucket name cannot be empty
    if s.is_empty() {
        return Err("Bucket name cannot be empty".into());
    }

    // Bucket names must be at least 3...
    if s.len() < 3 {
        return Err("Bucket name is too short".into());
    }

    // and no more than 63 characters long.
    if s.len() > 255 {
        return Err("Bucket name is too long".into());
    }

    Ok(())
}

/// Ensures that a given endpoint is valid, where valid means:
///   - Is not an empty string
///   - Is not an AWS endpoint
///   - Parses as a valid URL
#[cfg(feature = "s3")]
fn is_valid_endpoint(s: &str) -> Result<(), String> {
    // Endpoint cannot be an empty string
    if s.is_empty() {
        return Err("Endpoint cannot be empty".into());
    }

    // Endpoint must parse as a valid URL
    let url = match Url::parse(s) {
        Ok(u)  => Ok(u),
        Err(e) => Err(format!("Could not parse endpoint: {}", e)),
    }?;

    // We can only use HTTP or HTTPS URLs.
    match url.scheme() {
        "http" | "https" => Ok(()),
        scheme           => {
            Err(format!("URL scheme must be http or https, found {}", scheme))
        },
    }?;

    // Endpoint cannot be an AWS endpoint
    if let Some(hostname) = url.host_str() {
        if hostname.contains("amazonaws.com") {
            return Err("Endpoint cannot be used to specify AWS endpoints".into());
        }
    }

    Ok(())
}

/// Create the command line parser
fn create_app<'a>() -> App<'a> {
    debug!("Creating CLI app");

    let app = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("BUCKET")
                .env("S3DU_BUCKET")
                .hide_env_values(true)
                .index(1)
                .value_name("BUCKET")
                .help("Bucket to retrieve size of, retrieves all if not passed")
                .takes_value(true)
                .validator(is_valid_aws_s3_bucket_name)
        )
        .arg(
            Arg::new("MODE")
                .env("S3DU_MODE")
                .hide_env_values(true)
                .long("mode")
                .short('m')
                .value_name("MODE")
                .help("Use either CloudWatch or S3 to obtain bucket sizes")
                .takes_value(true)
                .default_value(DEFAULT_MODE)
                .possible_values(VALID_MODES)
        )
        .arg(
            Arg::new("REGION")
                .env("AWS_REGION")
                .hide_env_values(true)
                .long("region")
                .short('r')
                .value_name("REGION")
                .help("Set the AWS region to create the client in.")
                .takes_value(true)
                .default_value(&DEFAULT_REGION)
        )
        .arg(
            Arg::new("UNIT")
                .env("S3DU_UNIT")
                .hide_env_values(true)
                .long("unit")
                .short('u')
                .value_name("UNIT")
                .help("Sets the unit to use for size display")
                .takes_value(true)
                .default_value(DEFAULT_UNIT)
                .possible_values(VALID_SIZE_UNITS)
        );

    #[cfg(feature = "s3")]
    let app = app
        .arg(
            Arg::new("ENDPOINT")
                .env("S3DU_ENDPOINT")
                .hide_env_values(true)
                .long("endpoint")
                .short('e')
                .value_name("URL")
                .help("Sets a custom endpoint to connect to")
                .takes_value(true)
                .validator(is_valid_endpoint)
        )
        .arg(
            Arg::new("OBJECT_VERSIONS")
                .env("S3DU_OBJECT_VERSIONS")
                .hide_env_values(true)
                .long("object-versions")
                .short('o')
                .value_name("VERSIONS")
                .help("Set which object versions to sum in S3 mode")
                .takes_value(true)
                .default_value(DEFAULT_OBJECT_VERSIONS)
                .possible_values(OBJECT_VERSIONS)
        );

    app
}

/// Parse the command line arguments
pub fn parse_args() -> ArgMatches {
    debug!("Parsing command line arguments");

    create_app().get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_aws_s3_bucket_name() {
        let long_valid   = "a".repeat(65);
        let long_invalid = "a".repeat(256);

        let tests = vec![
            ("192.168.5.4",  true),
            ("no",           false),
            ("oh_no",        true),
            ("th1s-1s-f1n3", true),
            ("valid",        true),
            ("yes",          true),
            ("Invalid",      true),
            ("-invalid",     true),
            (&long_invalid,  false),
            (&long_valid,    true),
        ];

        for test in tests {
            let name  = test.0;
            let valid = test.1;

            let ret = is_valid_aws_s3_bucket_name(name.into());

            assert_eq!(ret.is_ok(), valid);
        }
    }

    #[cfg(feature = "s3")]
    #[test]
    fn test_is_valid_endpoint() {
        let tests = vec![
            ("https://s3.eu-west-1.amazonaws.com", false),
            ("https://minio.example.org/endpoint", true),
            ("http://minio.example.org/endpoint",  true),
            ("http://127.0.0.1:9000",              true),
            ("../ohno",                            false),
            ("minio.example.org",                  false),
            ("",                                   false),
            ("ftp://invalid.example.org",          false),
            ("ftp://no@invalid.example.org",       false),
            ("data:text/plain;invalid",            false),
            ("unix:/var/run/invalid.socket",       false),
        ];

        for test in tests {
            let url   = test.0;
            let valid = test.1;

            let ret = is_valid_endpoint(url.into());

            assert_eq!(ret.is_ok(), valid);
        }
    }
}
