# `s3du`

## v1.0.4

  - Update to [Rusoto] 0.44.0
    - Adds support for two new regions `af-south-1` (Africa), and `eu-south-1`
      (Milan).
  - Fully implement previously ignored tests, thanks to Rusoto adding
    `MultipleMockRequestDispatcher`.

## v1.0.3

  - Implement custom endpoints for S3 mode, which enables using `s3du` against
    other S3 compatible storage, such as [MinIO].
  - Improved AWS default region discovery by attempting to get the default
    region from the `AWS_DEFAULT_REGION` and `AWS_REGION` environment variables
    before falling back to `us-east-1`.
  - Fixed example IAM policies to include AWS S3 multipart upload permissions.

## v1.0.2

  - Make [chrono] an optional dependency, as it was only used by the
    `cloudwatch` mode.
  - Implement sizing of in-progress multipart uploads. Although they aren't
    really object verisons, the `--object-versions` arguments `all` and
    `multipart` account for the size of these incomplete objects.

## v1.0.1

  - Fix a potential issue where we might have tried to list a bucket we don't
    have access to.

## v1.0.0

  - Initial release

<!-- links -->
[chrono]: https://crates.io/crates/chrono
[MinIO]: https://min.io/
[Rusoto]: https://www.rusoto.org/
