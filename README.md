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

<!-- links -->
[`aws-vault`]: https://github.com/99designs/aws-vault/
[once per day]: https://docs.aws.amazon.com/AmazonS3/latest/dev/cloudwatch-monitoring.html
[AWS credentials]: https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html
[AWS CloudWatch]: https://aws.amazon.com/cloudwatch/
[AWS S3]: https://aws.amazon.com/s3/
