locals {
  parcel_files = fileset(path.module, "parcels/*")
}

variable "weigh_limit" {
  type = number
}
