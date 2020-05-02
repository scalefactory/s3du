# ARN for the account the s3du role will be assumed from
data aws_arn assuming_account {
  arn = "arn:aws:iam::${local.assuming_account_id}:root"
}

# AssumeRole policy
data aws_iam_policy_document assume_role {
  statement {
    sid    = "s3duAssumeRrole"
    effect = "Allow"

    actions = [
      "sts:AssumeRole",
    ]

    principals {
      type = "AWS"

      identifiers = [
        data.aws_arn.assuming_account.arn,
      ]
    }

    dynamic "condition" {
      for_each = local.require_mfa ? {"mfa" = true} : {}

      content {
        test     = "Bool"
        variable = "aws:MultiFactorAuthPresent"

        values = [
          "true",
        ]
      }
    }
  }
}

# CloudWatch policy
data aws_iam_policy_document cloudwatch {
  statement {
    sid    = "s3duCloudwatchAccess"
    effect = "Allow"

    actions = [
      "cloudwatch:GetMetricStatistics",
      "cloudwatch:ListMetrics",
    ]

    resources = [
      "*",
    ]

    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"

      values = [
        true,
      ]
    }
  }
}

# S3 policy
data aws_iam_policy_document s3 {
  statement {
    sid    = "s3duS3Access"
    effect = "Allow"

    actions = [
      "s3:GetBucketLocation",
      "s3:ListAllMyBuckets",
      "s3:ListBucket",
      "s3:ListBucketMultipartUploads",
      "s3:ListMultipartUploadParts",
    ]

    resources = [
      "*",
    ]

    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"

      values = [
        true,
      ]
    }
  }
}
