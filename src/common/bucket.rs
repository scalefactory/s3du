// Definition of a bucket
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use rusoto_core::Region;

/// Represents an S3 bucket.
///
/// This will always have a `name`.
#[derive(Debug)]
pub struct Bucket {
    /// The name of the S3 bucket.
    pub name: String,

    /// The region the S3 bucket lives in.
    ///
    /// This will currently only be used in S3 mode.
    pub region: Option<Region>,

    /// The storage types the bucket is using.
    ///
    /// This will currently only be used in CloudWatch mode.
    pub storage_types: Option<Vec<String>>,
}

/// Convenience type for a list of `Bucket`.
pub type Buckets = Vec<Bucket>;
