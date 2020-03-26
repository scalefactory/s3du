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
use log::debug;
use rusoto_core::Region;
use std::net::Ipv4Addr;
use std::str::FromStr;

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
const DEFAULT_REGION: &str = "us-east-1";

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
/// Valid S3 object versions for the `--s3-object-versions` switch.
#[cfg(feature = "s3")]
const S3_OBJECT_VERSIONS: &[&str] = &[
    "all",
    "current",
    "non-current",
];

/// Ensures that the AWS region that we're passed is valid.
///
/// There's a chance that this can be incorrect if AWS releases a region and
/// Rusoto lags behind on updating the Region list in `rusoto_core`.
fn is_valid_aws_region(s: String) -> Result<(), String> {
    match Region::from_str(&s) {
        Ok(_)  => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Ensures that a given bucket name is valid.
///
/// This validation is taken from
/// https://docs.aws.amazon.com/AmazonS3/latest/dev/BucketRestrictions.html,
/// specifically, the new standard.
/// This prevents us from sending API calls to AWS which will never be
/// serviced.
fn is_valid_aws_s3_bucket_name(s: String) -> Result<(), String> {
    // Bucket name cannot be empty
    if s.is_empty() {
        return Err("Bucket name cannot be empty".into());
    }

    // Bucket names must be at least 3...
    if s.len() < 3 {
        return Err("Bucket name is too short".into());
    }

    // and no more than 63 characters long.
    if s.len() > 63 {
        return Err("Bucket name is too long".into());
    }

    // Bucket names must not contain uppercase characters...
    for ch in s.chars() {
        if ch.is_uppercase() {
            return Err("Bucket names cannot contain uppercase chars".into());
        }
    }

    // or underscores.
    if s.contains("_") {
        return Err("Bucket names cannot contain underscores".into());
    }

    // Bucketnames must start with a lowercase letter or number
    // Unwrap should be safe here, we know we have a string > 0 characters.
    let ch = s.chars().nth(0).unwrap();
    if (!ch.is_ascii_lowercase() && !ch.is_ascii_alphanumeric()) || !ch.is_ascii_alphanumeric() {
        return Err("Bucket names must start with a lowercase char or number".into());
    }

    // Bucket names must not be formatted as an IP address (for example,
    // 192.168.5.4).
    if Ipv4Addr::from_str(&s).is_ok() {
        return Err("Bucket names cannot be formatted as an IP address".into());
    }

    Ok(())
}

/// Create the command line parser
fn create_app<'a, 'b>() -> App<'a, 'b> {
    debug!("Creating CLI app");

    let app = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("BUCKET")
                .env("S3DU_BUCKET")
                .hide_env_values(true)
                .index(1)
                .value_name("BUCKET")
                .help("Bucket to retrieve size of, retrieves all if not passed")
                .takes_value(true)
                .validator(is_valid_aws_s3_bucket_name)
        )
        .arg(
            Arg::with_name("MODE")
                .env("S3DU_MODE")
                .hide_env_values(true)
                .long("mode")
                .short("m")
                .value_name("MODE")
                .help("Use either CloudWatch or S3 to obtain bucket sizes")
                .takes_value(true)
                .default_value(DEFAULT_MODE)
                .possible_values(VALID_MODES)
        )
        .arg(
            Arg::with_name("REGION")
                .env("AWS_REGION")
                .hide_env_values(true)
                .long("region")
                .short("r")
                .value_name("REGION")
                .help("Set the AWS region to create the client in.")
                .takes_value(true)
                .default_value(DEFAULT_REGION)
                .validator(is_valid_aws_region)
        )
        .arg(
            Arg::with_name("UNIT")
                .env("S3DU_UNIT")
                .hide_env_values(true)
                .long("unit")
                .short("u")
                .value_name("UNIT")
                .help("Sets the unit to use for size display")
                .takes_value(true)
                .default_value(DEFAULT_UNIT)
                .possible_values(VALID_SIZE_UNITS)
        );

    #[cfg(feature = "s3")]
    let app = app.arg(
        Arg::with_name("OBJECT_VERSIONS")
            .env("S3DU_OBJECT_VERSIONS")
            .hide_env_values(true)
            .long("s3-object-versions")
            .short("o")
            .value_name("VERSIONS")
            .help("Set which object versions to sum in S3 mode")
            .takes_value(true)
            .default_value(DEFAULT_OBJECT_VERSIONS)
            .possible_values(S3_OBJECT_VERSIONS)
    );

    app
}

/// Parse the command line arguments
pub fn parse_args<'a>() -> ArgMatches<'a> {
    debug!("Parsing command line arguments");

    create_app().get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_core::Region;
    use std::str::FromStr;

    #[test]
    fn test_is_valid_aws_region() {
        let tests = vec![
            ("eu-central-1", true),
            ("eu-west-1", true),
            ("eu-west-2", true),
            ("int-space-station-1", false),
            ("nope-nope-42", false),
            ("us-east-1", true),
        ];

        for test in tests {
            let region = test.0;
            let valid  = test.1;

            let region = Region::from_str(region);

            assert_eq!(region.is_ok(), valid);
        }
    }

    #[test]
    fn test_is_valid_aws_s3_bucket_name() {
        let long_valid   = "a".repeat(63);
        let long_invalid = "a".repeat(64);

        let tests = vec![
            ("192.168.5.4",  false),
            ("no",           false),
            ("oh_no",        false),
            ("th1s-1s-f1n3", true),
            ("valid",        true),
            ("yes",          true),
            ("Invalid",      false),
            ("-invalid",     false),
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
}
