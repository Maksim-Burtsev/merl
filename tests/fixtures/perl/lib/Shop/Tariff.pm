package Shop::Tariff;
use Moose;

has 'rate' => (is => 'ro');
has weight => (is => 'rw');
has [qw(zone region)] => (is => 'ro');
has '+label';

sub describe {
    my $self = shift;
    my $text = <<"EOT";
sub heredoc_sub {
EOT
    my $more = <<~EOT;
        sub indented_sub {
        EOT
    my @list = qw(
        alpha
        beta
    );
    return $text . $more . q{
sub quoted_sub {
};
}

sub weigh {
    my ($self) = @_;
    state $calls = 0;
    $calls++;
#    ^ d: lib/Shop/Tariff.pm:28
#    status: calls → weigh::calls (local)
    return heredoc_sub() + indented_sub() + quoted_sub() + pod_sub() + data_sub() + zone();
#          ^ d: none
#                          ^ d: none
#                                           ^ d: none
#                                                          ^ d: none
#                                                                      ^ d: none
#                                                                                   ^ d: lib/Shop/Tariff.pm:6
}

=head1 NAME

sub pod_sub {

=cut

package Shop::Tariff::Inner {
    sub inner { 1 }
}

package Shop::Courier 1.02;

sub _discount { 0 }

1;
__END__
sub data_sub {
