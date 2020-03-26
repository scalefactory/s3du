// S3ObjectVersions
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use std::str::FromStr;

/// `S3ObjectVersions` represents which objects we're going to sum when
/// operating in S3 mode.
#[derive(Debug)]
pub enum S3ObjectVersions {
    /// Sum size of all object versions (both `Current` and `NonCurrent`)
    All,
    /// Sum only size of current objects
    Current,
    /// Sum only size of non-current objects
    NonCurrent,
}

/// This converts from the string argument we receive from the command line to
/// our enum type.
impl FromStr for S3ObjectVersions {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all"         => Ok(Self::All),
            "current"     => Ok(Self::Current),
            "non-current" => Ok(Self::NonCurrent),
            _             => Err("no match"),
        }
    }
}
