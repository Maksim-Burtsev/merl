use Test::More;
use Shop::Order;

my $self = Shop::Order->new(total => 5);
is($self->total, 5);
#   ^ d: t/order.t:4
#   status: self: local
Shop::Order::forward();
#            ^ d: none
done_testing;
