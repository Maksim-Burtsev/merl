local _M = {}

function _M.define(name)
  return setmetatable({
    name = name,
  }, { __index = _M })
end

function _M.paired(name)
  return setmetatable({}, { __index = _M }), name
end

function _M:enable()
  return self.name
end

local Other = {}

function Other:enable()
  return false
end

function _M.other()
  return setmetatable({}, { __index = Other })
end

return _M
