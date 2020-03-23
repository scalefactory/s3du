// Easily hadnle converting from a ListBucketsOutput into our own BucketList
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use crate::common::BucketNames;
use rusoto_s3::ListBucketsOutput;

pub struct BucketList(Vec<String>);

// Implement a conversion From ListBucketsOutput to BucketList
impl From<ListBucketsOutput> for BucketList {
    fn from(output: ListBucketsOutput) -> Self {
        let mut bucket_names = vec![];

        let buckets = match output.buckets {
            Some(buckets) => buckets,
            None          => vec![],
        };

        for bucket in buckets {
            if let Some(name) = bucket.name {
                bucket_names.push(name);
            }
        }

        BucketList(bucket_names)
    }
}

impl BucketList {
    // Return a reference to a vec of bucket names
    pub fn bucket_names(&self) -> &BucketNames {
        &self.0
    }

    // Filter our bucket list to only the one listed, if any.
    pub fn filter(&mut self, bucket: &str) {
        self.0.retain(|b| b == bucket)
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
