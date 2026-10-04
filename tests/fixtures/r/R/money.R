# The shop of #307 in R (#422): every d case carries its answer under it.
format_price <- function(cents) {
  sprintf("$%.2f", cents / 100)
}
rate <<- function(x) x * 0.2
square <- \(x) x^2
`%+%` <- function(a, b) paste(a, b)
`names<-.invoice` <- function(x, value) x
print.invoice <- function(x, ...) cat("invoice", format_price(x$total), "\n")
#                                                 ^ d: R/money.R:2
.onLoad <- function(libname, pkgname) invisible()
limit <- 10
TAX = 0.2
filter <- function(rows, keep) rows[keep]
weigh <- function(discount, grams) grams * (1 - discount)
