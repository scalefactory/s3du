// Easily hadnle converting from a ListBucketsOutput into our own BucketList
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use crate::common::BucketNames;
use rusoto_s3::ListBucketsOutput;

/// Holds a `Vec` of discovered S3 bucket names.
pub struct BucketList(BucketNames);

/// Implement a conversion from `rusoto_s3::ListBucketsOutput` to `BucketList`.
impl From<ListBucketsOutput> for BucketList {
    fn from(output: ListBucketsOutput) -> Self {
        let bucket_names = match output.buckets {
            Some(buckets) => {
                buckets.iter()
                    .filter_map(|b| b.name.to_owned())
                    .collect()
            },
            None => Vec::new(),
        };

        BucketList(bucket_names)
    }
}

impl BucketList {
    /// Return a reference to a `Vec` of `BucketNames`.
    pub fn bucket_names(&self) -> &BucketNames {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rusoto_s3::{
        Bucket,
        Owner,
    };

    #[test]
    fn test_bucketlist_from() {
        let buckets = vec![
            Bucket {
                creation_date: Some("2020-03-12T14:45:00.000Z".into()),
                name:          Some("a-bucket".into()),
            },
            Bucket {
                creation_date: Some("2020-03-11T14:45:00.000Z".into()),
                name:          Some("another-bucket".into()),
            },
        ];

        let owner = Owner {
            display_name: Some("aws".into()),
            id:           Some("1936a5d8a2b189cda450d1d1d514f3861b3adc2df515".into()),
        };

        let output = ListBucketsOutput {
            buckets: Some(buckets),
            owner:   Some(owner),
        };

        let bucket_list: BucketList = output.into();
        let mut bucket_names = bucket_list.bucket_names().to_owned();
        bucket_names.sort();

        let expected = vec![
            "a-bucket",
            "another-bucket",
        ];

        assert_eq!(bucket_names, expected);
    }
}
