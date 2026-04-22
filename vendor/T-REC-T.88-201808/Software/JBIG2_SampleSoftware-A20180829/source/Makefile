#
# Makefile for JBIG2 reference software.
# by Junichi HARA


INCLUDES = -I./ -I../include 

C_OPT = -DNDEBUG -Wall -Wno-uninitialized -Wno-deprecated $(KDU_GLIBS)

CFLAGS = $(INCLUDES) $(C_OPT) $(DEFINES)
LIBS = -lm -lpthread # You may comment out "-lpthreads" if coresys were non thread.

.SUFFIXES: .o .c .cpp

.cpp.o:
	$(CXX) -DNDEBUG -O2 $(CFLAGS) -o $@ -c $<
.c.o:
	$(CC) -O2 -DNDEBUG -std=c99 $(CFLAGS) -o $@ -c $<
.cpp.g:
	$(CXX) -g $(CFLAGS) -o $@ -c $<
.c.g:
	$(CC) -g -std=c99 $(CFLAGS) -o $@ -c $<

APP = jbig2
JZ1 = jb2z1
CMP = imgcomp

JB2SRC = codsub.cpp decsub.cpp JBIG2Common.cpp Jb2_Debug.cpp Jb2_MQLapper.cpp Jb2_T4T6Lapper.cpp Jbig2DEC.cpp Jbig2ENC.cpp MQ_codec.cpp T45_codec.cpp
#IMGSRC = ImageUtil.cpp getfilesize.cpp //comment out by Shigetaka Ogawa
IMGSRC = ImageUtil.cpp
APPSRC = JBIG2_Main.cpp
JZ1SRC = Jbig2_Z1main.cpp
CMPSRC = imgcomp.cpp

O_JB2 = $(JB2SRC:.cpp=.o)
O_IMG = $(IMGSRC:.cpp=.o)
O_APP = $(APPSRC:.cpp=.o)
O_JZ1 = $(JZ1SRC:.cpp=.o)
O_CMP = $(CMPSRC:.cpp=.o)

.PHONY: all clean test $(APP) $(JZ1) $(CMP)
all:	$(APP) $(JZ1) $(CMP)

clean:
	@echo "cleaning in the source directory"
	@rm -f $(APP) $(JZ1) $(CMP) $(APP).exe $(JZ1).exe $(CMP).exe *.tmp *.o

test:
	@(cd ../test; make test)

# The following ".o" object rules are comment-outted, because of the source codes make too many g++ warrnings.
#

#$(APP):	$(O_APP) $(O_JB2) $(O_IMG)
$(APP):	$(APPSRC) $(JB2SRC) $(IMGSRC)
	$(CXX) $(LIBS) -o $@ $^

#$(JZ1):	$(O_JZ1) $(O_JB2) $(O_IMG)
$(JZ1):	$(JZ1SRC) $(JB2SRC) $(IMGSRC)
	$(CXX) $(LIBS) -o $@ $^

#$(CMP):	$(O_CMP) $(O_IMG)
$(CMP):	$(CMPSRC) $(IMGSRC)
	$(CXX) $(LIBS) -o $@ $^
#
# DO NOT DELETE

codsub.o:         asmasm.h T4T6codec.h ImageUtil.h
decsub.o:         asmasm.h T4T6codec.h ImageUtil.h
ImageUtil.o:      ImageUtil.h asmasm.h getfilesize.h
JBIG2Common.o:    ImageUtil.h asmasm.h Jb2Common.h T4T6codec.h Jb2_MQLapper.h MQ_codec.h Jb2_T4T6Lapper.h Jb2_Debug.h
JBIG2_Main.o:     Jb2Common.h ImageUtil.h asmasm.h Jb2_Debug.h getfilesize.h
Jb2_Debug.o:      Jb2Common.h ImageUtil.h asmasm.h Jb2_Debug.h
Jb2_MQLapper.o:   asmasm.h Jb2Common.h Jb2_MQLapper.h MQ_codec.h Jb2_Debug.h Jb2_T4T6Lapper.h
Jb2_T4T6Lapper.o: ImageUtil.h asmasm.h Jb2_T4T6Lapper.h
Jbig2DEC.o:       ImageUtil.h asmasm.h Jb2Common.h T4T6codec.h Jb2_MQLapper.h MQ_codec.h Jb2_T4T6Lapper.h T45_codec.h Jb2_Debug.h
Jbig2ENC.o:       ImageUtil.h asmasm.h Jb2Common.h T4T6codec.h Jb2_MQLapper.h MQ_codec.h Jb2_T4T6Lapper.h T45_codec.h Jb2_Debug.h
Jbig2_Z1main.o:   ImageUtil.h asmasm.h Jb2Common.h Jb2_T4T6Lapper.h Jb2_MQLapper.h MQ_codec.h
MQ_codec.o:       ImageUtil.h asmasm.h MQ_codec.h Jb2_Debug.h
T45_codec.o:      ImageUtil.h asmasm.h Jb2_Debug.h T45_codec.h
#getfilesize.o:    getfilesize.h //comment out by Shigetaka Ogawa
imgcomp.o:        asmasm.h imgcomp.h ImageUtil.h
