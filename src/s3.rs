// s3du: A tool for informing you of the used space in AWS S3.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bucket_sizer;
mod bucket_list;
mod client;
pub use client::*;
