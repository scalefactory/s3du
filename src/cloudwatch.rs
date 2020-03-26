// Imports all of the components needed for cloudwatch::client
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// `BucketMetrics` handles returning the bucket names and storage types from
/// discovered CloudWatch metrics.
mod bucket_metrics;

/// Implementation of the `BucketSizer` trait for our CloudWatch `Client`.
mod bucket_sizer;

/// CloudWatch `Client`.
mod client;

pub use client::*;
