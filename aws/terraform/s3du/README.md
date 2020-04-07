# `s3du`

A Terraform module which configures an account with resources needed to run
`s3du` against its S3 buckets.

## Resources

The module will create the following resources:

  - IAM Role to be assumed by `s3du`
  - IAM Policy allowing use of some S3 APIs
  - IAM Policy allowing use of some CloudWatch APIs

The policies will only be attached to the IAM role when the appropriate
variables are set.

## Configuration

The module takes the following variables:

  - `assuming_account_id`: The account ID that will be permitted to assume the
    `s3du` role. This variable is required.
  - `enable_cloudwatch`: Enables the attachment of a policy allowing `s3du`
    access to CloudWatch APIs. Defaults to `false` (don't attach policy).
  - `enable_s3`: Enables the attachment of a policy allowing `s3du` access to
    S3 APIs. Defaults to `false` (don't attach policy).
  - `require_mfa`: Require MFA to be used to assume the `s3du` role. Defaults
    to `true`.
  - `role_name`: The name to give to the created `s3du` role. Defaults to
    `s3du`.
