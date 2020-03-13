// Command line interface parsing
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
use std::str::FromStr;

// Default mode that s3du runs in
// This catches cases where we've compiled with either:
//   - Only "cloudwatch"
//   - Both "cloudwatch" and "s3"
#[cfg(feature = "cloudwatch")]
const DEFAULT_MODE: &str = "cloudwatch";

// This catches cases where we've compiled with:
//   - Only "s3"
#[cfg(all(feature = "s3", not(feature = "cloudwatch")))]
const DEFAULT_MODE: &str = "s3";

// Default AWS region if one isn't provided on the command line
const DEFAULT_REGION: &str = "eu-west-1";

// This should match the string values in the ClientMode FromStr impl in main
const VALID_MODES: &[&str] = &[
    #[cfg(feature = "cloudwatch")]
    "cloudwatch",
    #[cfg(feature = "s3")]
    "s3",
];

// Ensures that the AWS region that we're passed is valid.
// There's a chance that this can be incorrect if AWS releases a region and
// Rusoto lags behind on updating the Region list in rusoto_core.
fn is_valid_aws_region(s: String) -> Result<(), String> {
    match Region::from_str(&s) {
        Ok(_)  => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

// Crate clap app
fn create_app<'a, 'b>() -> App<'a, 'b> {
    debug!("Creating CLI app");

    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
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
}

pub fn parse_args<'a>() -> ArgMatches<'a> {
    debug!("Parsing command line arguments");

    create_app().get_matches()
}

#[cfg(test)]
mod tests {
    use super::{
        FromStr,
        Region,
    };

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
}
