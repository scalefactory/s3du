# v1.0.2

  - Make [chrono] an optional dependency, as it was only used by the
    `cloudwatch` mode.
  - Implement sizing of in-progress multipart uploads. Although they aren't
    really object verisons, the `--object-versions` arguments `all` and
    `multipart` account for the size of these incomplete objects.

# v1.0.1

  - Fix a potential issue where we might have tried to list a bucket we don't
    have access to.

# v1.0.0

  - Initial release

<!-- links -->
[chrono]: https://crates.io/crates/chrono
