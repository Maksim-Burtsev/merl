source("R/money.R")
#          ^ d: R/money.R:1
library(dplyr)
#        ^ d: none
Cart <- R6::R6Class("Cart",
#            ^ d: none
  public = list(
#  ^ d: none
    total = 0,
#    ^ d: none
    label = function() format_price(self$total),
#                       ^ d: R/money.R:2
#                       status: format_price: by name, 1 match
#                                         ^ d: none
    add = function(cents) {
      self$total <- self$total + cents
#           ^ d: none
      invisible(self)
    }
  )
)
cart <- Cart$new()
#        ^ d: R/cart.R:5
cart$label()
#     ^ d: R/cart.R:11
cart$add(cents = 250)
#     ^ d: R/cart.R:15
#         ^ d: none
rows <- filter(1:3, c(TRUE, FALSE, TRUE))
#        ^ d: R/money.R:14
kept <- dplyr::filter(rows, TRUE)
#               ^ d: none
#               status: no definition for filter
own <- shop::format_price(250)
#             ^ d: R/money.R:2
joined <- 2 %+% 3
#            ^ d: R/money.R:7
sq <- square(4)
#      ^ d: R/money.R:6
taxed <- rate(5)
#         ^ d: R/money.R:5
print.invoice(list(total = 1))
#      ^ d: R/money.R:9
#                   ^ d: none
`names<-.invoice`(taxed, "a")
# ^ d: R/money.R:8
.onLoad("a", "b")
# ^ d: R/money.R:11
if (limit == TAX) limit <- 1
#    ^ d: R/money.R:12
#             ^ d: R/money.R:13
legacy_rate()
# ^ d: R/legacy.r:2
greet()
# ^ d: .Rprofile:2
handlers <- list(
  on_paid = function(order) format_price(order$total),
  on_lost = \(order) NULL
)
handlers$on_paid(list(total = 1))
#         ^ d: R/cart.R:57
handlers$on_lost(NULL)
#         ^ d: R/cart.R:58
