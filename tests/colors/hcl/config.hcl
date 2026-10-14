# A Nomad-style job, with every HCL construct the grammar knows.
// line comment
/* block
   comment */
job "web" {
  datacenters = ["dc1", "dc2"]
  type        = "service"
  priority    = 50
  ratio       = 0.75
  big         = 1e3
  enabled     = true
  disabled    = false
  nothing     = null

  group "app" {
    count = var.replicas > 0 ? var.replicas : 1

    task "server" {
      driver = "docker"
      env = {
        NAME    = "web-${local.suffix}"
        PORT    = 8080
        "quoted" = upper("x")
      }
      args = [for s in local.args : trimspace(s) if s != ""]
      ids  = { for k, v in local.map : k => v.id... }
      all  = local.items[*].name
      old  = local.items.*.name
      first = local.items[0]
      neg   = -1 + 2 * 3 / 4 % 5
      both  = !true && false || true
      cmp   = 1 <= 2 && 3 >= 2 && 1 == 1 && 2 != 3 && 1 < 2 && 2 > 1
      tmpl  = "%{ if local.on }on%{ else }off%{ endif }"
      loop  = "%{ for x in local.xs ~}${x},%{ endfor ~}"
      esc   = "tab\t quote\" dollar $${x} percent %%{y}"
      script = <<EOT
#!/bin/sh
echo "${local.name}"
EOT
      indented = <<-EOT
        hello ${var.who}
        EOT
      nested = jsonencode({ a = [1, 2], b = { c = "d" } })
      prov = provider::aws::arn_parse("arn")
    }
  }
}

single "line" { a = 1 }
empty {}
