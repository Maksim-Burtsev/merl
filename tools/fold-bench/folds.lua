-- nvim --clean --headless --cmd 'set rtp^=RT,QUERIES' -l folds.lua LANG LIST OUT
-- For each file of LIST: every line where nvim's treesitter folding starts a fold, and the last
-- line `zc` there hides. A file whose tree has an error is written with no folds and `error`.
vim.g._ts_force_sync_parsing = true
local lang, list, outpath = arg[1], arg[2], arg[3]
vim.treesitter.query.set(lang, 'injections', '')
local out = assert(io.open(outpath, 'w'))
for path in io.lines(list) do
  vim.cmd('silent! edit! ' .. vim.fn.fnameescape(path))
  local buf = vim.api.nvim_get_current_buf()
  if vim.bo.filetype == '' then vim.bo.filetype = lang end
  vim.treesitter.language.register(lang, vim.bo.filetype)
  local tree = vim.treesitter.get_parser(buf, lang):parse(true)[1]
  if tree:root():has_error() then
    out:write(path, '\terror\n')
  else
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
    vim.wo.foldlevel = 99
    vim.cmd('normal! zx')
    local folds = {}
    for l = 1, vim.api.nvim_buf_line_count(buf) do
      local level = vim.treesitter.foldexpr(l)
      if type(level) == 'string' and level:sub(1, 1) == '>' then
        vim.cmd(l .. 'foldclose')
        local h, e = vim.fn.foldclosed(l), vim.fn.foldclosedend(l)
        if h == l and e > l then folds[#folds + 1] = l .. ':' .. e end
        vim.cmd(l .. 'foldopen')
      end
    end
    out:write(path, '\t', table.concat(folds, ' '), '\n')
  end
  vim.cmd('silent! bwipeout!')
end
out:close()
