# Declaration-shaped lines inside literals declare nothing.
query <- "
select * from parcels
hidden_in_string <- function() 1
"
quoted <- 'it''s \' hidden_in_single <- 1
hidden_in_single <- 0'
raw <- r"(
hidden_in_raw <- function() 2
)"
dashed <- R"--[
hidden_in_dashed <- function() 3 ]" ]-"
]--"
# hidden_in_comment <- function() 4
#' hidden_in_roxygen <- 5
hidden_in_string()
# ^ d: none
hidden_in_single()
# ^ d: none
hidden_in_raw()
# ^ d: none
hidden_in_dashed()
# ^ d: none
hidden_in_comment()
# ^ d: none
after_literals <- function() 5
after_literals()
# ^ d: R/literals.R:26
