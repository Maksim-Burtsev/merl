local typedefs = {}

typedefs.uuid = Schema.define {
  type = "string",
}

typedefs.name = Schema.define({ type = "string" })

typedefs.count = typedefs.count or 0

setmetatable(typedefs, {
  __index = function(_, k)
    error(k)
  end,
})

return typedefs
