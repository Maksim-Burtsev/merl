-- #1126: the tables the third corner reaches, with namesakes of the standard library.
local M = {}

local Gaiwan = {}
Gaiwan.__index = Gaiwan

function Gaiwan.new(dose)
  return setmetatable({ dose = dose }, Gaiwan)
end

function Gaiwan:soak(secs)
  self.secs = secs
  return self
end

function Gaiwan:unfold()
  return self.dose
end

function Gaiwan:upper()
  return self.dose + 1
end

local Sencha = setmetatable({}, { __index = Gaiwan })
Sencha.__index = Sencha

function Sencha.new(dose)
  return setmetatable({ dose = dose }, Sencha)
end

function Sencha:unfold()
  return Gaiwan.unfold(self) * 2
end

function M.format(s)
  return s
end

function M.insert(list, v)
  list[#list + 1] = v
end

function M.whisk(n)
  return n
end

function string.steeped(s)
  return s .. "!"
end

M.Gaiwan = Gaiwan
M.Sencha = Sencha
return M
