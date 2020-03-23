// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use std::str::FromStr;

#[cfg(feature = "s3")]
#[derive(Debug)]
pub enum S3ObjectVersions {
    // Sum size of all object versions (both current and non-current)
    All,
    // Sum only size of current objects
    Current,
    // Sum only size of non-current objects
    NonCurrent,
}

#[cfg(feature = "s3")]
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
