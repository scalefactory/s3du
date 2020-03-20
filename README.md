# s3du

`s3du` is a tool which lets you know how much space your [AWS S3] buckets are
using according to [AWS CloudWatch].

Because `s3du` uses CloudWatch to obtain the bucket size, this means that there
could be up to a 24 hour latency on the reported size, vs. the actual size.
This is because CloudWatch is only updated with S3 bucket sizes [once per day].

In the future, an alternate mode to iterate over objects in a bucket and sum
the sizes may become available.

## Usage

`s3du` uses the default [AWS credentials] chain. As long as your AWS
credentials are available in some fashion, and your IAM user/role has the
correct permissions simply running `s3du` should return some results.

For example, if you manage your credentials with [`aws-vault`], you might run
`s3du` as follows:

```shell
aws-vault exec s3du-role -- s3du
```

By default, `s3du` will operate in the `eu-west-1` region. This can be
overridden either by the `AWS_REGION` environment variable, or the `--region`
CLI argument.

```shell
# Overriding the default AWS region with an environment variable
env AWS_REGION=us-east-1 s3du

# Overriding the default AWS region with a CLI arg
s3du --region=eu-central-1
```

## Features

The crate has two features, which are both enabled by default.

| Feature      | Purpose                      |
|--------------|------------------------------|
| `cloudwatch` | Enable use of CloudWatch API |
| `s3`         | Enable use of S3 API         |

`s3du` requires at least one of these features be enabled, attempting to
compile the crate with both features disabled will result in compilation
errors.

## AWS CloudWatch and AWS S3 Bucket Size Discrepancies

The CloudWatch and S3 modes will report sizes slightly differently. The
CloudWatch mode will always show the total bucket size, that is, it will show
the size of all current objects versions + non-current object versions. It is
not possible to change this behaviour.

The S3 mode will, by default, only show the bucket size for current object
versions. Command line flags (or environment variables) can be used to change
how the S3 mode operates. With these you can change the S3 mode to operate in
one of 3 ways:

  - All: Show bucket size as the sum of all current object versions + all
    non-current object versions.
  - Current: Show bucket size as the sum of all current object versions, this
    is the default.
  - NonCurrent: Show bucket size as the sum of all non-current object versions.

These can be selected via the `--s3-object-versions` CLI flag if `s3du` was
compiled with the `s3` feature.

## IAM Policies

In order to enable use of `s3du`, your IAM user or role will need one or both
of the following IAM policies attached, depending on which `s3du` modes you
wish to use.

### CloudWatch IAM Policy

This policy will enforce HTTPS use and will allow `s3du` access to the AWS
CloudWatch `GetMetricStatistics` and `ListMetrics` APIs. CloudWatch use will
be restricted to the `AWS/S3` namespace.

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Sid": "s3du-cloudwatch",
            "Effect": "Allow",
            "Action": [
                "cloudwatch:GetMetricStatistics",
                "cloudwatch:ListMetrics"
            ],
            "Resource": [
                "*"
            ],
            "Condition": {
                "Bool": {
                    "aws:SecureTransport": true
                },
                "StringEquals": {
                    "cloudwatch:namespace": "AWS/S3"
                }
            }
        }
    ]
}
```

### S3 IAM Policy

This policy will enforce HTTPS use and will allow `s3du` access to the AWS S3
`ListAllMyBuckets` and `ListBucket` APIs.

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Sid": "s3du-s3",
            "Effect": "Allow",
            "Action": [
                "s3:ListAllMyBuckets",
                "s3:ListBucket"
            ],
            "Resource": [
                "*"
            ],
            "Condition": {
                "Bool": {
                    "aws:SecureTransport": true
                }
            }
        }
    ]
}
```

<!-- links -->
[`aws-vault`]: https://github.com/99designs/aws-vault/
[once per day]: https://docs.aws.amazon.com/AmazonS3/latest/dev/cloudwatch-monitoring.html
[AWS credentials]: https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html
[AWS CloudWatch]: https://aws.amazon.com/cloudwatch/
[AWS S3]: https://aws.amazon.com/s3/
