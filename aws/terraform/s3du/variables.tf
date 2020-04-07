# The account ID that will be assuming this role.
variable assuming_account_id {
  description = "Account ID that will assume the s3du role"
  type        = string
}

# Bool toggling attachment of CloudWatch policies
variable enable_cloudwatch {
  description = "Enable attachment of CloudWatch related policies"
  type        = bool
  default     = false
}

# Bool toggling creation and attachment of S3 policies
variable enable_s3 {
  description = "Enable attachment of S3 related policies"
  type        = bool
  default     = false
}

# Require MFA presence to assume the role
variable require_mfa {
  description = "Require MFA be present to assume the role"
  type        = bool
  default     = true
}

# Name that the role should be given
variable role_name {
  description = "Role to assign the s3du role"
  type        = string
  default     = "s3du"
}
