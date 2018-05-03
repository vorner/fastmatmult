#!/usr/bin/perl
use common::sense;
use Data::Dumper;

my %data;

for my $f (@ARGV) {
	open my $in, '<', $f or die "Couldn't read $f: $!\n";

	for (<$in>) {
		if (/^(.*): (.*)$/) {
			$data{$1} = $2;
		}
	}
}

open my $out, '>', "arm.dat" or die "Couldn't write to arm.dat: $!\n";
for my $size (sort { $a <=> $b } keys %data) {
	print $out "$size\t$data{$size}\n";
}
undef $out;

$\ = ";\n";
print "set terminal svg size 400, 400 background rgb 'white'";
print "set output 'arm.svg'";
print "set log xyz";
print "set key right bottom";

print "plot 'arm.dat' title 'Armadillo' with linespoints lt 1 lc rgb 'red'";
