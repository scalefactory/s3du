# API Usage

Each of the methods for obtaining bucket sizes have different costs within AWS
as they will make differing numbers of API calls, and each API has its own
pricing model which may also vary by region.

The following is a rough guide to what `s3du` is doing so you can gauge what
the API usage may be.

### CloudWatch

AWS CloudWatch is the cheapest method of running `s3du`, at the cost of some
accuracy.

The CloudWatch mode of `s3du` will use at least 1 API call to perform the
`ListMetrics` call and at least 1 API call per S3 bucket for the
`GetMetricStatistics` call.

The reason these are listed as "at least 1" is because the API call results
could be paginated if the results lists are sufficiently long. `ListMetrics`
will paginate after 500 results while `GetMetricStatistics` will paginate after
1,440 statistics.

As a basic example, getting bucket sizes for an AWS account with 4 S3 buckets
in it should use 5 API calls total. 1 `ListMetrics` call to discover the
buckets and 4 `GetMetricStatistics` calls (one for each bucket).

### S3

AWS S3 is a more expensive, but more accurate, method of listing bucket sizes.

The S3 mode of `s3du` will use 1 API call to perform the `ListBuckets` API
call, 1 API call per listed bucket to `GetBucketLocation` to discover its
region, 1 API call per listed bucket to `HeadBucket` to make sure we have
access to list the objects, and:

  - at least 1 call to `ListMultipartUploads`, at least 1 call to
    `ListObjectVersions`, and at least 1 call to `ListParts` if in-progress
    multipart uploads are found in the `All` object mode
  - at least 1 call to `ListObjectsV2` per-bucket in the `Current` object
    (default) mode
  - at least 1 call to `ListObjectVersions` per bucket in the `NonCurrent`
    object mode
  - at least 1 call to `ListMultipartUploads` per-bucket in the `Multipart`
    mode with at least 1 call to `ListParts` if any in-progress multipart
    uploads are found

Each of the API calls listed above will return 1,000 objects maximum, if your
bucket has more objects than this, pagination will be required.

For example, let's say we're running in S3 mode getting the sizes of `current`
object versions and our AWS account has 2 buckets.
`bucket-a` (no versioning enabled) has 10,000 objects and `bucket-b`
(versioning enabled) has 32,768 object versions of which 13,720 are current
versions and 19,048 are non-current versions. There is also an in-progress
multipart upload with 2 parts uploaded in `bucket-a`. This would mean:

  - 1 API call to `ListBuckets` for bucket discovery
  - 2 API calls to `GetBucketLocation` for region discovery, 1 for each bucket
  - 2 API calls to `HeadBucket` to check we have access, 1 for each bucket
  - 10 API calls to `ListObjectsV2` for `bucket-a`
  - 14 API calls to `ListObjectsV2` for `bucket-b`

for a total of 29 API calls.

If we were to run `s3du` against the same account a second time, but ask for
the sum of `all` object versions, we'd get the following:

  - 1 API call to `ListBuckets` for bucket discovery
  - 2 API calls to `GetBucketLocation` for region discovery, 1 for each bucket
  - 2 API calls to `HeadBucket` to check we have access, 1 for each bucket
  - 10 API calls to `ListObjectVersions` for `bucket-a`
  - 33 API calls to `ListObjectVersions` for `bucket-b`
  - 1 API call to `ListMultipartUploads` for `bucket-a`
  - 1 API call to `ListMultipartUploads` for `bucket-b`
  - 1 API call to `ListParts` for `bucket-a`

for a total of 51 API calls.

A third run of `s3du` against the same account but asking for the sum of
`non-current` object versions would result in the following:

  - 1 API call to `ListBuckets` for bucket discovery
  - 2 API calls to `GetBucketLocation` for region discovery, 1 for each bucket
  - 2 API calls to `HeadBucket` to check we have access, 1 for each bucket
  - 1 API calls to `ListObjectVersions` for `bucket-a`
  - 33 API calls to `ListObjectVersions` for `bucket-b`

for a total of 39 API calls.

You will notice that the number of API calls to `ListObjectVersions` for
`bucket-b` are the same across both the `all` and `non-current` object versions
requests, this is because any filtering for current vs. non-current objects in
these scenarios must be done by `s3du`. The `ListObjectVersions` API does not
let us specify which object versions we'd like to retrieve.
