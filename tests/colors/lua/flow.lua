-- Control flow, operators and strings.
local function sum(list)
  local total = 0
  for i = 1, #list do
    total = total + list[i]
  end
  for _, v in ipairs(list) do
    if not v or v ~= v then goto continue end
    total = total - v // 2 % 3
    ::continue::
  end
  while total > 100 and total < 1000 do
    total = total / 2
  end
  repeat
    total = total << 1 | 1 & ~0
  until total >= 10
  return total
end

local long = [[
a long string
with two lines]]
local escaped = "tab\there\n\"quoted\" \65"
local single = 'it\'s'
--[[ a block
comment ]]
local words = {}
for word in ("a b c"):gmatch("%a+") do
  words[#words + 1] = word:upper()
end
local trimmed = string.gsub("  x  ", "^%s*(.-)%s*$", "%1")
local ok, err = pcall(function() return sum({ 1, 2, 3 }) end)
print(ok, err, trimmed, long .. escaped, single, coroutine.running(), self, debug.traceback())
do
  local co = coroutine.create(function(a, b) coroutine.yield(a + b) end)
  print(coroutine.resume(co, 1, 2))
end
