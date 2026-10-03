use v5.38;
use experimental 'class';

class Shop::Coupon;

field $discount :param = 0;
field @codes;

method apply($price) {
    return $price - $discount;
#           ^ d: lib/Shop/Coupon.pm:9
#           status: price: local
#                    ^ d: lib/Shop/Coupon.pm:6
}

method codes { scalar @codes }
#                      ^ d: lib/Shop/Coupon.pm:7
