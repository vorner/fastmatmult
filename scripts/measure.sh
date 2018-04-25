#!/bin/sh

set -ex

export RUSTFLAGS='-C target-cpu=native'
export CARGO_INCREMENTAL=

run() {
	SIZE=$1
	CHEAP=$2

	if [ -f "$SIZE.out" ] ; then
		echo "$SIZE already exists"
	else
		echo "Running $SIZE"
		cargo run --release --bin manip -- generate $SIZE $SIZE a.in
		cargo run --release --bin manip -- generate $SIZE $SIZE b.in
		cargo run --release --bin measure -- $CHEAP a.in b.in | tee tmp.out
		mv tmp.out $SIZE.out
	fi
}

for i in 256 512 1024 2048 4096 ; do
	run $i
done

for i in 8192 16384 ; do
	run $i --cheap
done
