BOB := $(wildcard *.bob)
SVG := $(patsubst %.bob,%.svg,$(BOB)) \
	arm.svg buldozer.svg celeron.svg xeon.svg beast.svg

all: $(SVG)

%.svg: %.bob
	svgbob $< -o $@

arm.svg: ../measured/buldozer-4.2GHz-8/armadillo.out ../scripts/crunch.pl
	../scripts/crunch.pl $< | gnuplot

define graph
$1.svg: $$(wildcard ../measured/$2/*.out) ../scripts/plot.pl
	cd ../measured/$2/ && ../../scripts/plot.pl | gnuplot
	mv ../measured/$2/graph.svg $$@
endef

$(eval $(call graph,buldozer,buldozer-4.2GHz-8))
$(eval $(call graph,celeron,celeron-1.8GHz-4))
$(eval $(call graph,xeon,xeon-3.5GHz-4_2))
$(eval $(call graph,beast,xeon-1.8GHz-20_2))

.PHONY: all
