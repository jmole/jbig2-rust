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
//#include	<math.h>
//#include	<malloc.h>
#include	"ImageUtil.h"
#include	"Jb2_Debug.h"
#include	"T45_codec.h"


byte4 *T45_Dec( struct StreamChain_s *str )
{
	uchar	ccc, numCmpts;
	uchar	CmptsL;
	ubyte4	num_Val;
	byte4	*Col;
	ubyte4	Run_Length;
	ubyte4	i, kkk;

	numCmpts = Ref_1Byte( str );
	CmptsL   = Ref_1Byte( str );
	num_Val      = (ubyte4)Ref_4Byte( str );
	Col = new byte4 [num_Val*numCmpts];
	if(CmptsL==1){
		i=0;
		do{
			Run_Length = Ref_1Byte( str );
			if(!Run_Length){
				Run_Length = Ref_1Byte( str) ;
				Run_Length<<=8;
				Run_Length += (ubyte4)Ref_1Byte( str );
			}
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				Col[i*numCmpts+ccc] = Ref_1Byte( str );
			for( kkk=1 ; kkk<Run_Length ; kkk++){
				for(ccc=0 ; ccc<numCmpts ; ccc++ )
					Col[(i+kkk)*numCmpts+ccc] = Col[i*numCmpts+ccc];
			}
			i += Run_Length;
		}while(i<num_Val);
	}
	else if(CmptsL==2){
		i=0;
		do{
			Run_Length = Ref_1Byte( str );
			if(!Run_Length){
				Run_Length = Ref_1Byte( str) ;
				Run_Length<<=8;
				Run_Length += (ubyte4)Ref_1Byte( str );
			}
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				Col[i*numCmpts+ccc] = Ref_2Byte( str );
			for( kkk=1 ; kkk<Run_Length ; kkk++){
				for(ccc=0 ; ccc<numCmpts ; ccc++ )
					Col[(i+kkk)*numCmpts+ccc] = Col[i*numCmpts+ccc];
			}
			i += Run_Length;
		}while(i<num_Val);
	}
	else if(CmptsL==4){
		i=0;
		do{
			Run_Length = Ref_1Byte( str );
			if(!Run_Length){
				Run_Length = Ref_1Byte( str) ;
				Run_Length<<=8;
				Run_Length += (ubyte4)Ref_4Byte( str );
			}
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				Col[i*numCmpts+ccc] = Ref_2Byte( str );
			for( kkk=1 ; kkk<Run_Length ; kkk++){
				for(ccc=0 ; ccc<numCmpts ; ccc++ )
					Col[(i+kkk)*numCmpts+ccc] = Col[i*numCmpts+ccc];
			}
			i += Run_Length;
		}while(i<num_Val);
	}

	return	Col;
}

struct StreamChain_s *T45_Enc( struct StreamChain_s *str, byte4 *Col, uchar numCmpts, uchar CmptsL, byte4 num_Val )
{
	uchar	ccc;
	ubyte4	Run_Length=1;
	byte4	i;

	str = Stream1ByteWrite( str,  numCmpts, str->buf_length );
	str = Stream1ByteWrite( str,  CmptsL, str->buf_length );
	str = Stream4ByteWrite( str,  num_Val, str->buf_length, BIG_ENDIAN );
	if(CmptsL==1){
		for( i=0 ; i<num_Val ;){
			str = Stream1ByteWrite( str,  Run_Length, str->buf_length );
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				str = Stream1ByteWrite( str,  Col[i*numCmpts+ccc], str->buf_length );
			i += Run_Length;
		}
	}
	else if(CmptsL==2){
		for( i=0 ; i<num_Val ; ){
			str = Stream1ByteWrite( str,  Run_Length, str->buf_length );
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				str = Stream2ByteWrite( str,  Col[i*numCmpts+ccc], str->buf_length, BIG_ENDIAN );
			i += Run_Length;
		}
	}
	else if(CmptsL==4){
		for( i=0 ; i<num_Val ; ){
			str = Stream1ByteWrite( str,  Run_Length, str->buf_length );
			for(ccc=0 ; ccc<numCmpts ; ccc++ )
				str = Stream4ByteWrite( str,  Col[i*numCmpts+ccc], str->buf_length, BIG_ENDIAN );
			i += Run_Length;
		}
	}

	return	str;
}
