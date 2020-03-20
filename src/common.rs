// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;
use rusoto_core::Region;
use std::str::FromStr;

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

// Client configuration
#[derive(Debug)]
pub struct ClientConfig {
    pub mode:   ClientMode,
    pub region: Region,
    #[cfg(feature = "s3")]
    pub s3_object_versions: S3ObjectVersions,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            mode:   ClientMode::CloudWatch,
            region: Region::UsEast1,
            #[cfg(feature = "s3")]
            s3_object_versions: S3ObjectVersions::Current,
        }
    }
}

// Valid modes that s3du can operate in.
#[derive(Debug, Eq, PartialEq)]
pub enum ClientMode {
    #[cfg(feature = "cloudwatch")]
    CloudWatch,
    #[cfg(feature = "s3")]
    S3,
}

// This is used to work out which mode we're in after parsing the CLI.
// We shouldn't ever hit the error condition here.
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
