local W = {}

local Courier = {}
Courier.__index = Courier

function Courier.new(name)
  return setmetatable({ name = name }, Courier)
end

function Courier:dispatch()
  return self.name
end

function W.weigh(grams)
  return grams // 1000
end

W.Courier = Courier
return W
