// Imports all of the components needed for cloudwatch::client
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bucket_metrics;
mod bucket_sizer;
mod client;

pub use client::*;
