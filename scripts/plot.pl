#!/usr/bin/perl
use common::sense;
use Data::Dumper;

my %data;

for my $f (glob '*.out') {
	open my $in, '<', $f or die "Couldn't read '$f': $!\n";

	my ($fbase) = ($f =~ /([^.]*)/);

	for (<$in>) {
		chomp;
		if (/^(.*): (.*)$/) {
			my ($size, $name) = ($fbase, $1);
			my $t = $2;
			if ($size eq 'armadillo') {
				# The armadillo is in a separate file with sizes instead of names
				($size, $name) = ($name, $size);
			}
			$data{$name}->{$size} = $t;
		}
	}
}

my @colors = qw(red blue black orchid green brown purple olivegreen orange #83ffd5 #007f00 #8a0000);
my $cnum;

$\ = ";\n";
print "set terminal svg size 700, 450 background rgb 'white'";
print "set output 'graph.svg'";
print "set log xyz";
print "set key right bottom";
print "set xlabel \"Side of the matrix\"";
print "set ylabel \"Time (seconds)\"";

while (my ($algo, $data) = each %data) {
	open my $out, '>', "$algo.dat" or die "Couldn't write $algo.dat: $!\n";
	my $data = $data{$algo};
	for my $size (sort { $a <=> $b } keys %$data) {
		print $out "$size\t$data->{$size}\n";
	}
}

sub conv($) {
	my ($name) = @_;
	$name =~ s/_/\\_/g;
	return $name;
}

my %algos = (
	Armadillo => 'armadillo',
	Simple => 'simple',
	SIMD => 'simd',
	Recursive => 'recursive-16',
	Parallel => 'recursive-paral-cutoff-16',
	Combined => 'recursive-simd-paral-cutoff-256',
	Strassen => 'strassen-256',
);

print "plot " . join ', ', (map "'$algos{$_}.dat' title '".conv($_)."' with linespoints lt 1 lc rgb \"".$colors[$cnum ++ % scalar @colors]."\"", sort keys %algos);
