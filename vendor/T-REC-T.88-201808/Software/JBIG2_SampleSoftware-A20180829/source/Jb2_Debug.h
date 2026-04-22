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




#ifndef	_JB2_DEBUG_H_
#define _JB2_DEBUG_H_

#define		JBIG2_DEBUG00	0
#define		JBIG2_DEBUG01	0	//Symboldic Symbol-Image output(Enc/Dec common)
#define		JBIG2_DEBUG02	0	//MQ_EncImage/MQ_DecImage CX Debug
#define		JBIG2_DEBUG03	0	//GenericRegion generic-Image output
#define		JBIG2_DEBUG04	0	//PageEnd Image output
#define		JBIG2_DEBUG05	1	//TextRegion Image output
#define		JBIG2_DEBUG06	0	//MQ_Integer Test

#if ( JBIG2_DEBUG01 || JBIG2_DEBUG03 || JBIG2_DEBUG05 )
static	char	GenCounter=0;
static	char	SymCounter=0;
static	char	TxtCounter=0;
#endif

void Jb2_Debug_Print( char *fname, struct ImageChain_s *ImageChain, byte4 NumChain, uchar ColorExtFlag);
void Jb2_Sym_Debug_Print( struct Jb2_ImageChain_s *ImageChain, byte4 ID, byte4 SegNo);
#endif
