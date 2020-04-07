# Role for s3du to assume
resource aws_iam_role s3du {
  name               = local.role_name
  description        = "Role assumed by s3du"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
  tags               = local.tags
}

# CloudWatch policy attachment
resource aws_iam_role_policy_attachment cloudwatch {
  count = local.enable_cloudwatch ? 1 : 0

  role       = aws_iam_role.s3du.name
  policy_arn = aws_iam_policy.cloudwatch.arn
}

# S3 policy attachment
resource aws_iam_role_policy_attachment s3 {
  count = local.enable_s3 ? 1 : 0

  role       = aws_iam_role.s3du.name
  policy_arn = aws_iam_policy.s3.arn
}
