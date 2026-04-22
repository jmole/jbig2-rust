/*************************************************************************/
/** Copyright (c) 2016-2018 ICT-Link Corporation                         **/
/**                                                                      **/
/** Written by Shigetaka Ogawa (Japan)                                   **/
/**       s_ogawa@mug.biglobe.ne.jp                                      **/
/**************************************************************************/
/*
This software module is an implementation of one or more tools as proposed
for the JBIG2 standard.

The copyright in this software is being made available under the
license included below. This software may be subject to other third
party and contributor rights, including patent rights, and no such
rights are granted under this license.

This software module was originally contributed by the party as
listed below in the course of development of the ISO/IEC 14492 (JBIG2)
 standard and the Rec.ITU-T T.88 standard for validation and reference purposes:

- ICT-Link

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:
  * Redistributions of source code must retain the above copyright notice,
    this list of conditions and the following disclaimer.
  * Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions and the following disclaimer in the documentation
    and/or other materials provided with the distribution.
  * Neither the name of the ICT-Link nor the names of its
    contributors may be used to endorse or promote products derived from this
    software without specific prior written permission.
  * Redistributed products derived from this software must conform to
    ISO/IEC 14492 (JBIG2) except that non-commercial redistribution
    for research and for furtherance of ISO/IEC standards is permitted.
    Otherwise, contact the contributing parties for any other
    redistribution rights for products derived from this software.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*************************************************************************/





#include	<stdio.h>
#include	<stdlib.h>
#include	<string.h>
#include	"Jb2Common.h"
#include	"ImageUtil.h"
#include	"Jb2_Debug.h"



void Jb2_Debug_Print( char *fname, struct ImageChain_s *ImageChain, byte4 NumChain, uchar ColorExtFlag)
{
	struct	Image_s *rImage;
	if(NumChain>=0){
		ImageChain = ImageChainSearch( ImageChain, NumChain );
		if(!ColorExtFlag){
			rImage = (struct Image_s *)ImageCharToBit1(ImageChain->Image);
			SaveBmp777(fname, rImage);
			delete	[]	rImage->Pdata;
			delete	rImage;
		}
		else
			SaveBmp777(fname, ImageChain->Image);
	}
}

void Jb2_Sym_Debug_Print( struct Jb2_ImageChain_s *ImageChain, byte4 ID, byte4 SegNo )
{
	struct	Image_s *rImage;
	char	fname[256];
	byte4	Counter00, Counter10;

	strcpy(fname, "ImageSym000Seg000");	
	Counter10   = ID/10;
	Counter00   = ID%10;
	fname[0x09] = 0x30+Counter10;
	fname[0x0a] = 0x30+Counter00;
	Counter10   = SegNo/10;
	Counter00   = SegNo%10;
	fname[0x0f]=0x30+Counter10;
	fname[0x10]=0x30+Counter00;
	strcat(fname, ".bmp");
	ImageChain = Jb2_ImageChainSearch( ImageChain, ID );
	rImage = (struct Image_s *)ImageCharToBit1(ImageChain->Image);
	SaveBmp777(fname, rImage);
	delete	[]	rImage->Pdata;
	delete	rImage;
}

