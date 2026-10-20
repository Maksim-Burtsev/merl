-- #1062: `S.rinse` is a field of this table, which a bare `rinse` elsewhere cannot reach.
local S = {}

function S.rinse()
  return 1
end

return S
