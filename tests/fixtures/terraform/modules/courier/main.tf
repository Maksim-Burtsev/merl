resource "aws_sqs_queue" "parcels" {
  name = "parcels-${var.region}"
  #                     ^ d: modules/courier/variables.tf:1
  delay_seconds = var.rate + var.coupon_rate
  #                   ^ d: modules/courier/variables.tf:5
  #                              ^ d: modules/courier/variables.tf:9
  tags = { cap = "${path.module}" }
  #                 ^ d: none
}

output "queue_name" {
  value = aws_sqs_queue.parcels.name
  #       ^ d: modules/courier/main.tf:1
}

resource "aws_sqs_queue" "overweight" {
  name          = "overweight"
  delay_seconds = var.weigh_limit
  #                   ^ d: modules/courier/files.tf:5
}
