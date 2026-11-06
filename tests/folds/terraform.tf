# Every construct `f` folds in Terraform, and the cases that could break it. # f: none
terraform { # f: 2-9
  required_version = ">= 1.3" # f: 2-9
  required_providers { # f: 4-8
    aws = { # f: 5-7
      source = "hashicorp/aws" # f: 5-7
    } # f: 5-7
  } # f: 4-8
} # f: 2-9
 # f: none
/* A block comment # f: 11-13
   with a { brace } and a "quote # f: 11-13
*/ # f: 11-13
resource "aws_iam_policy" "this" { # f: 14-42
  name = "a } in a string, and \"an escaped { quote\"" # f: 14-42
  # a { brace in a comment # f: 14-42
  // and another } # f: 14-42
  policy = <<-EOT # f: 18-23
    { # f: 18-23
      "Statement": [ # f: 18-23
        "${var.name}" # f: 18-23
      ] # f: 18-23
    EOT
  user_data = <<EOF # f: 24-26
} # f: 24-26
EOF
  subnets = [ # f: 14-42
    "a", # f: 14-42
    "b", # f: 14-42
  ] # f: 14-42
  tags = merge(var.tags, { # f: 31-34
    Name = "x" # f: 31-34
    Ips  = "[${join(", ", formatlist("\"%s}\"", local.ips))}]" # f: 31-34
  }) # f: 31-34
  by_name = { # f: 14-42
    for k, v in var.items : k => v # f: 14-42
  } # f: 14-42
  dynamic "ingress" { # f: 38-41
    for_each = var.rules # f: 38-41
    content {} # f: 38-41
  } # f: 38-41
} # f: 14-42
 # f: none
locals { one = { a = 1 } } # f: none
