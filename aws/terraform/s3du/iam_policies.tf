# CloudWatch policy
resource aws_iam_policy cloudwatch {
  name        = "s3du-cloudwatch-access"
  description = "Allow s3du access to CloudWatch APIs"
  policy      = data.aws_iam_policy_document.cloudwatch.json
}

# S3 policy
resource aws_iam_policy s3 {
  name        = "s3du-s3-access"
  description = "Allow s3du access to S3 APIs"
  policy      = data.aws_iam_policy_document.s3.json
}
