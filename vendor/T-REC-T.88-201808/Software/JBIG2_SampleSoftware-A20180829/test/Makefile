#
# Makefile for JBIG2 reference software.
# by Junichi HARA

APP = jbig2
CMP = imgcomp

OBJDIR = ../source

SYSTEM = $(shell uname -s | cut -c 1-5)

APPCMD = $(shell if [ $(SYSTEM) = MINGW ]; then echo $(OBJDIR)/$(APP).exe; else echo $(OBJDIR)/$(APP); fi)
CMPCMD = $(shell if [ $(SYSTEM) = MINGW ]; then echo $(OBJDIR)/$(CMP).exe; else echo $(OBJDIR)/$(CMP); fi)

.SUFFIXES: .bmp .jb2 .ini

.PHONY: all test
all:	test

clean:
	@echo "no cleaning in test directory"

test: $(APPCMD) $(CMPCMD)
	@echo "T1: no parameter file test"
	$(APPCMD) -i codeStreamTest1_TT1 -f jb2 -o codeStreamTest1_TT1_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT1_TT00 -F bmp -m mse
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT1_TT01 -F bmp -m mse
	$(CMPCMD) -t codeStreamTest2 -f bmp -T codeStreamTest1_TT1_TT02 -F bmp -m mse

	@echo "T2: parameter-2 file test"
	$(APPCMD) -i codeStreamTest1 -f bmp -o codeStreamTest1_TT2 -F jb2 -ini jbig2_Param2.ini
	$(APPCMD) -i codeStreamTest1_TT2 -f jb2 -o codeStreamTest1_TT2_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT2_TT00 -F bmp -m mse

	@echo "T3: parameter-3 file test"
	$(APPCMD) -i codeStreamTest1 -f bmp -o codeStreamTest1_TT3 -F jb2 -ini jbig2_Param3.ini
	$(APPCMD) -i codeStreamTest1_TT3 -f jb2 -o codeStreamTest1_TT3_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT3_TT00 -F bmp -m mse

	@echo "T4: parameter of Generic Region Template=1 test"
	$(APPCMD) -i codeStreamTest1 -f bmp -o codeStreamTest1_TT4 -F jb2 -ini jbig2_Param4.ini
	$(APPCMD) -i codeStreamTest1_TT4 -f jb2 -o codeStreamTest1_TT4_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT4_TT00 -F bmp -m mse

	@echo "T5: parameter of SymbolDictionary Refinement symbol definite test"
	$(APPCMD) -i codeStreamTest1 -f bmp -o codeStreamTest1_TT5 -F jb2 -ini jbig2_Param5.ini
	$(APPCMD) -i codeStreamTest1_TT5 -f jb2 -o codeStreamTest1_TT5_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT5_TT00 -F bmp -m mse

	@echo "T6: parameter of TextRegionSegment Symbol Refinement set test "
	$(APPCMD) -i codeStreamTest2 -f bmp -o codeStreamTest2_TT6 -F jb2 -ini jbig2_Param6.ini
	$(APPCMD) -i codeStreamTest2_TT6 -f jb2 -o codeStreamTest2_TT6_TT -F bmp
	$(CMPCMD) -t codeStreamTest2 -f bmp -T codeStreamTest2_TT6_TT00 -F bmp -m mse

	@echo "T7: parameter of AMD2 test"
	$(APPCMD) -i codeStreamTest1 -f bmp -o codeStreamTest1_TT7 -F jb2 -ini jbig2_Param7.ini
	$(APPCMD) -i codeStreamTest1_TT7 -f jb2 -o codeStreamTest1_TT7_TT -F bmp
	$(CMPCMD) -t codeStreamTest1 -f bmp -T codeStreamTest1_TT7_TT00 -F bmp -m mse

	@echo "T8: parameter of AMD3 test"
	$(APPCMD) -i codeStreamTest3 -f bmp -o codeStreamTest3_TT8 -F jb2 -ini jbig2_Param8.ini
	$(APPCMD) -i codeStreamTest3_TT8 -f jb2 -o codeStreamTest3_TT8_TT -F bmp
	$(CMPCMD) -t codeStreamTest3 -f bmp -T codeStreamTest3_TT8_TT00 -F bmp -m mse

	@echo "T9: parameter-9 file test"
	$(APPCMD) -i F01_200 -f bmp -o F01_200_TT9 -F jb2 -ini jbig2_Param9.ini
	$(APPCMD) -i F01_200_TT9 -f jb2 -o F01_200_TT9_TT -F bmp
	$(CMPCMD) -t F01_200 -f bmp -T F01_200_TT9_TT00 -F bmp -m mse

	@echo "T10: test with F01_200 image test"
	$(APPCMD) -i F01_200 -f bmp -o F01_200_TT10 -F jb2
	$(APPCMD) -i F01_200_TT10 -f jb2 -o F01_200_TT10_TT -F bmp
	$(CMPCMD) -t F01_200 -f bmp -T F01_200_TT10_TT00 -F bmp -m mse
#
$(APPCMD):
	(cd $(OBJDIR); make $(APP))

$(CMPCMD):
	(cd $(OBJDIR); make $(CMP))