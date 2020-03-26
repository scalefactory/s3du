// ClientMode
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use std::str::FromStr;

/// Valid modes that `s3du` can operate in.
#[derive(Debug, Eq, PartialEq)]
pub enum ClientMode {
    /// CloudWatch mode is available when compiled with the `cloudwatch`
    /// feature.
    #[cfg(feature = "cloudwatch")]
    CloudWatch,

    /// S3 mode is available when compiled with the `s3` feature.
    #[cfg(feature = "s3")]
    S3,
}

/// This is used to work out which mode we're in after parsing the CLI.
/// We shouldn't ever hit the error condition here.
impl FromStr for ClientMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "cloudwatch")]
            "cloudwatch" => Ok(Self::CloudWatch),
            #[cfg(feature = "s3")]
            "s3"         => Ok(Self::S3),
            _            => Err("no match"),
        }
    }
}
