#[[ Broken on purpose: see NOTES.md ]]
set(A "unclosed
if(A
  message(STATUS ${A)
endif(
function()
foreach(x IN LISTS)
