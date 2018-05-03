BOB := $(wildcard *.bob)
SVG := $(patsubst %.bob,%.svg,$(BOB)) arm.svg

all: $(SVG)

%.svg: %.bob
	svgbob $< -o $@

arm.svg: ../measured/buldozer-4.2GHz-8/armadillo.out ../scripts/crunch.pl
	../scripts/crunch.pl $< | gnuplot

.PHONY: all
