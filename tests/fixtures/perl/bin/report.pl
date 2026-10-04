use strict;
use lib 'lib';
use Shop::Order;
#         ^ d: lib/Shop/Order.pm:1
use File::Basename qw(basename);
#                     ^ d: local/lib/perl5/File/Basename.pm:5
#                     status: basename: via import File::Basename
#         ^ d: local/lib/perl5/File/Basename.pm:1
use Shop::Tariff qw(weigh);
use Shop::Coupon;

my $self  = basename($0);
#           ^ d: local/lib/perl5/File/Basename.pm:5
#           status: basename: via import File::Basename
my $order = Shop::Order->new(total => 12);
#                 ^ d: lib/Shop/Order.pm:1
#           ^ d: lib/Shop/Order.pm:1
#                        ^ d: lib/Shop/Order.pm:14
#                        status: new: via Shop::Order
print $order->total, "\n";
#             ^ d: lib/Shop/Order.pm:25
#             status: total: by name, 1 match
#      ^ d: bin/report.pl:15
#      status: order: local
print Shop::Order::total($order), "\n";
#                  ^ d: lib/Shop/Order.pm:25
#                  status: total: via Shop::Order
print $Shop::Order::VERSION, "\n";
#                   ^ d: lib/Shop/Order.pm:5
#                   status: VERSION: via Shop::Order
print Shop::Order::MAX_ITEMS, "\n";
#                  ^ d: lib/Shop/Order.pm:10
#                  status: MAX_ITEMS: via Shop::Order
my $tariff = Shop::Tariff->new(rate => 2);
#                          ^ d: picker lib/Shop/Order.pm:14
#                          status: new: by name, 1 match
print $tariff->rate, $tariff->weight, $tariff->label, "\n";
#              ^ d: lib/Shop/Tariff.pm:4
#                             ^ d: lib/Shop/Tariff.pm:5
#                                              ^ d: lib/Shop/Tariff.pm:7
print weigh($tariff), Shop::Tariff::Inner::inner(), "\n";
#     ^ d: lib/Shop/Tariff.pm:26
#     status: weigh: via import Shop::Tariff
#                                          ^ d: lib/Shop/Tariff.pm:48
#                                          status: inner → Shop::Tariff::Inner::inner (via Shop::Tariff::Inner)
#                                   ^ d: lib/Shop/Tariff.pm:47
print Shop::Coupon->new(discount => 3)->apply(10), "\n";
#           ^ d: lib/Shop/Coupon.pm:4
#                                       ^ d: lib/Shop/Coupon.pm:9
print $self, "\n";
#      ^ d: bin/report.pl:12
#      status: self: local
for my $item (1 .. 3) {
    print $item;
#          ^ d: bin/report.pl:53
#          status: item: local
}
my %seen;
$seen{x} = 1;
#^ d: bin/report.pl:58
#status: seen: local
print 'total', "\n";
