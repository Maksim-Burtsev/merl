region  = "us-east-1"
enabled = true
tags = {
  team = "core"
  env  = "prod"
}
disks = [
  { name = "/dev/sdb", size = 100 },
]
