# A basket of the shop: its globals beside the report's locals.
m = 1
courier = north
#         ^ d: src/tariff.jl:26

basket(items) = [weigh(i) for i in items]
#                ^ d: src/tariff.jl:52
#                             ^ d: scripts/basket.jl:6

function checkout(items, coupon)
#                        ^ d: scripts/basket.jl:10
    total = 0
    for (i, item) in enumerate(items)
#           ^ d: scripts/basket.jl:13
        total += gross(coupon, item)
#       ^ d: scripts/basket.jl:12
#                ^ d: src/tariff.jl:48
#                              ^ d: scripts/basket.jl:13
    end
    discount = rate(coupon)
#              ^ d: picker src/tariff.jl:30, src/tariff.jl:33
    cmd = `ls
    function shell() end
    `
    pattern = r"function regex() end"
    raw"struct Raw end"
    hidden(); shell(); regex(); Hidden(); Raw()
#   ^ d: none
#             ^ d: none
#                      ^ d: none
#                               ^ d: none
#                                         ^ d: none
    total - discount + m
#   ^ d: scripts/basket.jl:12
#           ^ d: scripts/basket.jl:20
#                      ^ d: scripts/basket.jl:2
end
