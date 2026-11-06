-- Rebound names (#1393): a name bound again on the cursor's own line, before the cursor, by a
-- parameter, a loop variable, a `local` after `;` or a reassignment, is not the outer binding;
-- the control rows keep master's answer on a plain read, a closed block, a comment and a string.
local Flask = {}
Flask.__index = Flask
function Flask.new() return setmetatable({}, Flask) end
function Flask:glug() return 1 end
local Jug = {}
Jug.__index = Jug
function Jug.new() return setmetatable({}, Jug) end
function Jug:glug() return 2 end

local flask = Flask.new()
local function swig() return 1 end

local function plain()
  return flask:glug()
  --           ^ d: lua/jug/rebound.lua:7
  --     ^ d: lua/jug/rebound.lua:13
end

local function param_one_line()
  local flask = Flask.new()
  local f = function(flask) return flask:glug() end
  --                                     ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                               ^ d: lua/jug/rebound.lua:24
  return flask:glug()
  --           ^ d: lua/jug/rebound.lua:7
  --     ^ d: lua/jug/rebound.lua:23
end

local function loop_one_line(jugs)
  local flask = Flask.new()
  for _, flask in ipairs(jugs) do flask:glug() end
  --                                    ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                              ^ d: lua/jug/rebound.lua:34
end

local function callback(jugs)
  local flask = Flask.new()
  each(jugs, function(flask) return flask:glug() end)
  --                                      ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                                ^ d: lua/jug/rebound.lua:41
end

local function local_after_semicolon()
  local flask = Jug.new(); return flask:glug()
  --                                    ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                              ^ d: lua/jug/rebound.lua:47
end

local function local_in_loop()
  while true do local flask = Jug.new(); flask:glug() end
  --                                           ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                                     ^ d: lua/jug/rebound.lua:53
end

local function local_in_repeat()
  repeat local flask = Jug.new() until flask:glug()
  --                                         ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                                   ^ d: lua/jug/rebound.lua:59
end

local function reassigned()
  local flask = Flask.new()
  flask = Jug.new(); flask:glug()
  --                       ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
end

local function local_shadows_function()
  local swig = 2; return swig
  --                     ^ d: lua/jug/rebound.lua:71
end

function Flask:own_self()
  local f = function(self) return self:glug() end
  --                                   ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  return self:glug()
  --          ^ d: lua/jug/rebound.lua:7
end

local function read_before_binding()
  local flask = flask:glug()
  --                  ^ d: lua/jug/rebound.lua:7
  --            ^ d: lua/jug/rebound.lua:13
end

local function closed_block()
  do local flask = Jug.new() end; return flask:glug()
  --                                           ^ d: lua/jug/rebound.lua:7
  --                                     ^ d: lua/jug/rebound.lua:13
end

local function closed_function()
  local f = function(flask) return 1 end; return flask:glug()
  --                                                   ^ d: picker lua/jug/rebound.lua:7, lua/jug/rebound.lua:11
  --                                             ^ d: picker lua/jug/rebound.lua:13
end

local function in_string()
  local s = "flask = Jug.new()"; return flask:glug()
  --                                          ^ d: lua/jug/rebound.lua:7
  --                                    ^ d: lua/jug/rebound.lua:13
end

local function in_comment()
  return flask:glug() -- local flask = Jug.new(); flask:glug()
  --           ^ d: lua/jug/rebound.lua:7
  --     ^ d: lua/jug/rebound.lua:13
end

local function plain_read_later()
  local g = function(x) return x end
  return flask:glug()
  --           ^ d: lua/jug/rebound.lua:7
  --     ^ d: lua/jug/rebound.lua:13
end
