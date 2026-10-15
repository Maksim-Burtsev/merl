# A small ledger: entries, totals and a report.
library(dplyr)

MAX_ENTRIES <- 1024L
ratio <- 1.5e3
imaginary <- 2i
mask <- 0xFF
missing_values <- c(NA, NA_integer_, NaN, Inf, -Inf, NULL)
flags <- c(TRUE, FALSE, T)
name <- "ledger \"main\"\n"
raw <- r"(C:\path\to\file)"
single <- 'it\'s'

new_entry <- function(name, amount = 0, ...) {
  structure(list(name = name, amount = amount, tags = list(...)), class = "entry")
}

is_credit = function(e) e$amount > 0 && nchar(e$name) != 0

total <- function(entries) {
  sum <- 0
  for (e in entries) {
    if (is.null(e$amount)) next
    sum <- sum + e$amount
  }
  return(sum)
}

describe <- \(e) {
  if (is_credit(e)) {
    "credit"
  } else if (e$amount == 0) {
    "empty"
  } else {
    "debit"
  }
}

report <- function(entries) {
  n <- length(entries)
  if (n > MAX_ENTRIES) stop("too many entries: ", n)
  i <- 1
  while (TRUE) {
    if (i > n) break
    cat(sprintf("%-20s %8.2f\n", entries[[i]]$name, entries[[i]]$amount))
    i <- i + 1
  }
  repeat {
    break
  }
  invisible(entries)
}

entries <- list(new_entry("rent", -900), new_entry("salary", 3000, "work"))
summary <- entries |>
  vapply(\(e) e$amount, numeric(1)) |>
  stats::median()
totals <- data.frame(name = c("a", "b"), amount = c(1, 2)) %>%
  dplyr::filter(amount >= 1, !is.na(name)) %>%
  mutate(double = amount * 2, share = amount / sum(amount))
mat <- matrix(1:6, nrow = 2) %*% t(matrix(1:6, nrow = 2))
model <- lm(amount ~ name, data = totals)
setClass("Account", representation(owner = "character"))
acct <- new("Account", owner = "me")
acct@owner
x <<- 5
10 -> y
ledger:::internal_helper
function(..1) ..1
