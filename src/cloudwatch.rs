// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bucket_metrics;
mod bucket_sizer;
mod client;

pub use client::*;
