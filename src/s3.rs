// Imports all of the components needed for s3::client
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Implementation of the `BucketSizer` trait for our S3 `Client`.
mod bucket_sizer;

/// S3 `Client`.
mod client;

pub use client::*;
