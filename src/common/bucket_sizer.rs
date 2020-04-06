// BucketSizer trait
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;
use super::{
    Bucket,
    Buckets,
};

/// `BucketSizer` represents the required methods to list S3 buckets and find
/// their sizes.
///
/// This trait should be implemented by all `Client`s performing these tasks.
#[async_trait]
pub trait BucketSizer {
    /// Returns a list of bucket names.
    async fn buckets(&self) -> Result<Buckets>;

    /// Returns the size of the given `bucket` in bytes.
    async fn bucket_size(&self, bucket: &Bucket) -> Result<usize>;
}
