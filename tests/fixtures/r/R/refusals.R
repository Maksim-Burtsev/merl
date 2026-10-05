# What declares nothing: a wrapped call's named arguments beside a top-level `=`.
discount = 0.1
weigh(
# ^ d: R/money.R:15
  discount = 0.5,
#  ^ d: R/refusals.R:2
  grams = 250
#  ^ d: none
)
if (discount == 0) weigh(grams = 1, discount = 0)
#    ^ d: R/refusals.R:2
#                         ^ d: none
settings <- list()
settings$timeout <- 30
#         ^ d: none
# ^ d: R/refusals.R:13
settings[["retries"]] <- 3
#           ^ d: none
names(settings) <- c("timeout", "retries")
# ^ d: none
fit <- lm(weight ~ height, data = settings)
#          ^ d: none
#                   ^ d: none
30 -> courier
#      ^ d: none
for (parcel in seq_len(3)) print(parcel)
#                                 ^ d: none
format_price(10)
