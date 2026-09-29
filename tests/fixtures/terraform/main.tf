# The shop of #307: every d case carries its answer in a comment under it.
# A comment that reads like code declares nothing:
# resource "aws_s3_bucket" "baskets" {

locals {
  rate_cap = 100
  gross    = var.tariff_rate * local.rate_cap
  #              ^ d: variables.tf:2
  #                                  ^ d: main.tf:6
  tags = {
    rate_cap = "shop"
  }
}

data "aws_iam_policy_document" "courier" {
  statement {
    actions = ["s3:GetObject"]
  }
}

resource "aws_s3_bucket" "baskets" {
  bucket = "shop-${var.region}"
  #                   ^ d: variables.tf:12
  tags   = local.tags
  #              ^ d: main.tf:10
}

resource "aws_s3_bucket_policy" "baskets" {
  bucket = aws_s3_bucket.baskets.id
  #                      ^ d: main.tf:21
  #                              ^ d: main.tf:21
  policy = data.aws_iam_policy_document.courier.json
  #                                      ^ d: main.tf:15
}

module "courier" {
  source      = "./modules/courier"
  region      = var.region
  rate        = local.gross
  #                   ^ d: main.tf:7
  coupon_rate = var.coupon_rate
  #                 ^ d: variables.tf:7
}

resource "aws_sns_topic" "dispatch" {
  name   = module.courier.queue_name
  #               ^ d: main.tf:36
  count  = length(var.tariff_rate)
  policy = <<EOT
resource "aws_sns_topic" "coupon_rate" {
}
EOT
  tags = { index = count.index }
  #                ^ d: none
}

output "gross" {
  value = local.gross
}

resource "aws_sqs_queue" "overweight" {
  delay_seconds = var.weigh_limit
  #                   ^ d: none
}
