# `s3du`

## v1.2.0

  - Switch from [Rusoto] to the official [AWS SDK Rust]
  - Update [clap] to 4.x
  - Bump MSRV to 1.74.0
  - Switch from [lazy_static] to [once_cell]
  - Simplify code in CloudWatch mode removing the need for [chrono]

## v1.1.0

  - Update [Rusoto] to 0.46.0
  - Update [Tokio] to 1.0
  - Bump MSRV to 1.46.0
  - Add [Rayon] parallel iteration when sizing S3 objects

## v1.0.6

  - Bump MSRV to 1.41.0

## v1.0.5

  - Update to [Rusoto] 0.45.0

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
[clap]: https://crates.io/crates/clap
[lazy_static]: https://crates.io/crates/lazy_static
[once_cell]: https://crates.io/crates/once_cell
[AWS SDK Rust]: https://github.com/awslabs/aws-sdk-rust
[MinIO]: https://min.io/
[Rayon]: https://crates.io/crates/rayon
[Rusoto]: https://www.rusoto.org/
[Tokio]: https://tokio.rs/
