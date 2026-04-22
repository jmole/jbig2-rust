#
# Makefile for JBIG2 reference software.
# by Junichi HARA

.PHONY: all jbig2 imgcomp test clean

all:	jbig2 imgcomp test

jbig2:
	@(cd source; make jbig2)

imgcomp:
	@(cd source; make imgcomp)

test:	jbig2 imgcomp
	@(cd test; make test)

clean:
	@(cd source; make clean)
	@(cd test; make clean)

