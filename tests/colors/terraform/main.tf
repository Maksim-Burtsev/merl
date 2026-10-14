terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

variable "region" {
  type        = string
  default     = "eu-west-1"
  description = "The region"
}

variable "tags" {
  type = map(string)
}

variable "spec" {
  type = object({ name = string, ports = list(number), on = bool, any = any })
}

locals {
  name   = "app-${terraform.workspace}"
  root   = path.root
  module = path.module
  cwd    = path.cwd
  ids    = [for s in data.aws_subnets.all.ids : s]
}

data "aws_subnets" "all" {
  filter {
    name   = "vpc-id"
    values = [var.vpc_id]
  }
}

resource "aws_instance" "web" {
  count         = var.enabled ? 2 : 0
  ami           = data.aws_ami.ubuntu.id
  instance_type = "t3.micro"
  subnet_id     = element(local.ids, count.index)
  tags          = merge(var.tags, { Name = "${local.name}-${count.index}" })

  dynamic "ebs_block_device" {
    for_each = var.disks
    content {
      device_name = ebs_block_device.value.name
    }
  }

  lifecycle {
    create_before_destroy = true
    ignore_changes        = [tags]
  }
}

module "vpc" {
  source = "./modules/vpc"
  cidr   = "10.0.0.0/16"
}

output "ip" {
  value     = aws_instance.web[*].public_ip
  sensitive = false
}

output "from_module" {
  value = module.vpc.id
}
