local M = {}

local config = {  -- f: 3-6
  name = "end",
  tags = { "if", "then" },
}

--[[
function not_code()
end
]]

local doc = [==[
if this were code
end
]==]

function M.setup(opts)  -- f: 18-40
  opts = opts or {}  -- f: 18-40
  if opts.debug then  -- f: 20-27
    print("debug: end")  -- f: 18-40
  elseif opts.quiet then  -- f: 22-23
    print('quiet')  -- f: 18-40
  else  -- f: 24-26
    print("normal")  -- f: 18-40
    print("still")  -- f: 18-40
  end  -- f: 18-40
  for i = 1, 10 do  -- f: 28-30
    print(i)  -- f: 18-40
  end  -- f: 18-40
  while opts.busy do  -- f: 31-33
    opts.tick()  -- f: 18-40
  end  -- f: 18-40
  repeat  -- f: 34-36
    opts.tick()  -- f: 18-40
  until opts.done  -- f: 18-40
  do  -- f: 37-39
    local x = 1  -- f: 18-40
  end  -- f: 18-40
end  -- f: 18-40

local function call(  -- f: 42-49
  a,  -- f: 42-49
  b  -- f: 42-49
)  -- f: 42-49
  return M.run(a, function(x)  -- f: 46-48
    return x + b -- end  -- f: 46-48
  end)  -- f: 46-48
end  -- f: 42-49

M.handlers = setmetatable({}, {  -- f: 51-55
  __index = function(t, k)  -- f: 52-54
    return k  -- f: 52-54
  end,  -- f: 52-54
})

return M
