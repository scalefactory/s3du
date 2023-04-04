// cli: This module is responsible for command line interface parsing
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use clap::{
    crate_description,
    crate_name,
    crate_version,
    Arg,
    ArgAction,
    ArgMatches,
    Command,
};
use clap::builder::PossibleValuesParser;
use log::debug;
use once_cell::sync::Lazy;
use std::env;

#[cfg(feature = "s3")]
use http::Uri;

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

/// Default AWS region if one isn't provided on the command line.
///
/// Obtains the default region in the following order:
///   - `AWS_DEFAULT_REGION` environment variable
///   - `AWS_REGION` environment variable
///   - Falls back to `us-east-1` if regions in the environment variables
///     are unavailable
static DEFAULT_REGION: Lazy<String> = Lazy::new(|| {
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
            |region| region,
        );

    region.to_string()
});

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
/// <https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html>.
/// We validate based on the legacy standard for compatibility.
fn is_valid_aws_s3_bucket_name(s: &str) -> Result<String, String> {
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

    Ok(s.to_string())
}

/// Ensures that a given endpoint is valid, where valid means:
///   - Is not an empty string
///   - Is not an AWS endpoint
///   - Parses as a valid URL
#[cfg(feature = "s3")]
fn is_valid_endpoint(s: &str) -> Result<String, String> {
    // Endpoint cannot be an empty string
    if s.is_empty() {
        return Err("Endpoint cannot be empty".into());
    }

    // Endpoint must parse as a valid URL
    let uri = match Uri::try_from(s) {
        Ok(u)  => Ok(u),
        Err(e) => Err(format!("Could not parse endpoint: {e}")),
    }?;

    // We can only use HTTP or HTTPS URLs.
    let scheme = match uri.scheme_str() {
        Some(scheme) => Ok(scheme),
        None         => Err("No URI scheme found")
    }?;

    match scheme {
        "http" | "https" => Ok(()),
        scheme           => {
            Err(format!("URI scheme must be http or https, found {scheme}"))
        },
    }?;

    // Endpoint cannot be an AWS endpoint
    if let Some(hostname) = uri.host() {
        if hostname.contains("amazonaws.com") {
            return Err("Endpoint cannot be used to specify AWS endpoints".into());
        }
    }

    Ok(s.to_string())
}

/// Create the command line parser
fn create_app() -> Command {
    debug!("Creating CLI app");

    // Below is a little odd looking, as we try to specify an argument order
    // but also have some options behind features.
    let app = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::new("BUCKET")
                .action(ArgAction::Set)
                .env("S3DU_BUCKET")
                .help("Bucket to retrieve size of, retrieves all if not passed")
                .hide_env_values(true)
                .index(1)
                .value_name("BUCKET")
                .value_parser(is_valid_aws_s3_bucket_name)
        );

    #[cfg(feature = "s3")]
    let app = app
        .arg(
            Arg::new("ENDPOINT")
                .action(ArgAction::Set)
                .env("S3DU_ENDPOINT")
                .help("Sets a custom endpoint to connect to")
                .hide_env_values(true)
                .long("endpoint")
                .short('e')
                .value_name("URL")
                .value_parser(is_valid_endpoint)
        );

    let app = app.arg(
            Arg::new("MODE")
                .action(ArgAction::Set)
                .default_value(DEFAULT_MODE)
                .env("S3DU_MODE")
                .help("Use either CloudWatch or S3 to obtain bucket sizes")
                .hide_env_values(true)
                .long("mode")
                .short('m')
                .value_name("MODE")
                .value_parser(PossibleValuesParser::new(VALID_MODES))
        );

    #[cfg(feature = "s3")]
    let app = app
        .arg(
            Arg::new("OBJECT_VERSIONS")
                .action(ArgAction::Set)
                .default_value(DEFAULT_OBJECT_VERSIONS)
                .env("S3DU_OBJECT_VERSIONS")
                .help("Set which object versions to sum in S3 mode")
                .hide_env_values(true)
                .long("object-versions")
                .short('o')
                .value_name("VERSIONS")
                .value_parser(PossibleValuesParser::new(OBJECT_VERSIONS))
        );

    app.arg(
            Arg::new("REGION")
                .action(ArgAction::Set)
                .default_value(&**DEFAULT_REGION)
                .env("AWS_REGION")
                .help("Set the AWS region to create the client in.")
                .hide_env_values(true)
                .long("region")
                .short('r')
                .value_name("REGION")
        )
        .arg(
            Arg::new("UNIT")
                .action(ArgAction::Set)
                .default_value(DEFAULT_UNIT)
                .env("S3DU_UNIT")
                .help("Sets the unit to use for size display")
                .hide_env_values(true)
                .long("unit")
                .short('u')
                .value_name("UNIT")
                .value_parser(PossibleValuesParser::new(VALID_SIZE_UNITS))
        )
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
