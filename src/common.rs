// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;
use std::str::FromStr;

mod clientconfig;
pub use clientconfig::*;
mod clientmode;
pub use clientmode::*;

// These are used by the CloudWatch and S3 modes.
pub type BucketNames = Vec<String>;

// BucketSizer trait implemented by cloudwatch and s3 mods
#[async_trait]
pub trait BucketSizer {
    // Takes a bucket name and returns the bucket size in bytes
    async fn bucket_size(&self, bucket: &str) -> Result<usize>;
    // Returns a list of bucket names
    async fn list_buckets(&mut self) -> Result<BucketNames>;
}

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

#[derive(Debug)]
pub enum SizeUnit {
    Binary,
    Bytes,
    Decimal,
}

impl FromStr for SizeUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binary"  => Ok(Self::Binary),
            "bytes"   => Ok(Self::Bytes),
            "decimal" => Ok(Self::Decimal),
            _         => Err("no match"),
        }
    }
}
