// Imports all of the components needed for s3::client
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bucket_sizer;
mod bucket_list;
mod client;

pub use client::*;
