BOB := $(wildcard *.bob)
SVG := $(patsubst %.bob,%.svg,$(BOB))

all: $(SVG)

%.svg: %.bob
	svgbob $< -o $@

.PHONY: all
