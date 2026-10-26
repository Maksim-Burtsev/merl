---@class CaddyModule
---@field seal fun(lid: string)
local C = setmetatable({}, {
  __index = function(_, k)
    return function()
      return k
    end
  end,
})

return C
