# expect: lang="Perl" total=11 code=7 comment=2 blank=2 shebang=0
use strict;
use warnings;
use Test::More;

# Basic arithmetic checks.
my $sum = 2 + 2;
is($sum, 4, "two plus two");
ok($sum > 3, "greater than three");

done_testing();
