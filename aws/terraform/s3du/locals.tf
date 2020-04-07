locals {
  assuming_account_id = var.assuming_account_id
  enable_cloudwatch   = var.enable_cloudwatch
  enable_s3           = var.enable_s3
  require_mfa         = var.require_mfa
  role_name           = var.role_name

  tags = {
    Managed_by = "terraform"
  }
}
