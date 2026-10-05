package Shop::Order;
use strict;
use warnings;

our $VERSION = '1.02';
our @EXPORT_OK = qw(order_total);

use constant PI => 3.14;
use constant {
    MAX_ITEMS => 10,
    MIN_ITEMS => 1,
};

sub new {
    my ($class, %args) = @_;
    my $self = { total => $args{total} // 0 };
#                          ^ d: lib/Shop/Order.pm:15
#                          status: args → new::args (local)
    return bless $self, $class;
#                 ^ d: lib/Shop/Order.pm:16
#                 status: self → new::self (local)
#                        ^ d: lib/Shop/Order.pm:15
}

sub total { $_[0]{total} }

sub order_total($self) {
    return $self->total * PI;
#           ^ d: lib/Shop/Order.pm:27
#           status: self: local
#                 ^ d: lib/Shop/Order.pm:25
#                 status: total: by name, 1 match
#                         ^ d: lib/Shop/Order.pm:8
}

sub _private {
    my $self = shift;
    local $VERSION = 2;
#          ^ d: lib/Shop/Order.pm:5
    return $self->_discount(MIN_ITEMS);
#           ^ d: lib/Shop/Order.pm:37
#                           ^ d: lib/Shop/Order.pm:11
#                 ^ d: lib/Shop/Tariff.pm:53
}

sub Shop::Order::discount {
    my ($self, @items) = @_;
    return $items[0] + $#items;
#           ^ d: lib/Shop/Order.pm:47
#           status: items → Shop::Order::discount::items (local)
#                        ^ d: lib/Shop/Order.pm:47
    return 0 unless $self->{total};
#                           ^ d: none
#                    ^ d: lib/Shop/Order.pm:47
}

sub forward;

1;
