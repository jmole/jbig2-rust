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
#include	<string.h>
//#include	<malloc.h>
#include	"ImageUtil.h"
#include	"MQ_codec.h"

void InitMQ_Codec( struct mqcodec_s *codec, struct StreamChain_s *str, byte4 numCX, char enc_dec, byte4 Eaddr, uchar KindOfCode )
{
	byte4	CX;

	codec->Areg=0x8000;
	codec->Creg=0;
	codec->B_buf=0;
	codec->ctreg=12;
	codec->first_flag=0;
	codec->numCX = numCX;
	memset(codec->index,0,sizeof(uchar)*numCX);
	if(KindOfCode==JPEG2000){
		CX=18;		
		codec->index[CX]=46;	//CX_UNI
		CX=16;
		codec->index[CX]=3;		//CX_RUN
		CX=0;
		codec->index[CX]=4;		//
	}

	if(enc_dec==DEC){
		codec->Creg = (str->buf[str->cur_p]<<16);
		str->cur_p++;
		MQ_ByteIn(codec, str, Eaddr);
		codec->Creg<<=7;
		codec->ctreg-=7;
	}
}

//MQ Enc(JBIG2 & JPEG2000)
struct StreamChain_s *Enc_MQ(struct StreamChain_s *str, struct	mqcodec_s *codec, uchar Di, byte4 CX, char DebugF )
{
	uchar	index;
	uchar	MPS_D;
	byte4	Qe;



	//codec->index
	index = codec->index[ CX ]&0x7f;
	MPS_D = (codec->index[ CX ]&0x80);
	//QeIndexTable
	Qe =  QeIndexTable[index] &0xffff ;

#if 0
	if(DebugF)
		printf("%d,%d,%d,%d,%d\n",CX, index, Qe, codec->Areg, Di);
#endif


	codec->Areg -= Qe;
	Di = (Di<<7);



	if(Di ^ MPS_D ){																//code LPS
		if(codec->Areg < Qe)	codec->Creg += Qe;									//C2
		else					codec->Areg = Qe;									//C1
		codec->index[ CX ]  = ((QeIndexTable[index]>>24)^MPS_D);					//C1,C2
	}
	else{																			//code MPS
		if(codec->Areg < Qe){
			codec->Areg  = Qe;
			codec->index[ CX ] = (uchar)((QeIndexTable[index]>>16) | MPS_D);		//C5
		}
		else{
			codec->Creg += Qe;														//C3
			if( !(codec->Areg & 0x8000) )
				codec->index[ CX ] = (uchar)((QeIndexTable[index]>>16) | MPS_D);	//C4
		}
	}

	//Renorm
	for( ;!(codec->Areg&0x8000) ; codec->Areg<<= 1){
		codec->Creg  <<= 1;
		codec->ctreg--;
		if(!codec->ctreg)
			str = MQ_ByteOut(codec, str);
	}
	return	str;
}

uchar Dec_MQ( struct StreamChain_s *str, struct mqcodec_s *codec, byte4 CX, byte4 Eaddr, char DebugF )
{
	uchar	index;
	byte4	Qe;
	byte4	Chigh;
	uchar	Di;
	uchar	MPS_D;

	/*********メインルーチン***********************************************/
	index = codec->index[ CX ]&0x7f;
	MPS_D = codec->index[ CX ]&0x80;
	Qe =  QeIndexTable[index] &0xffff ;
#if 0
	if(DebugF)
		printf("%d,%d,%d,%d,",CX, index, Qe, codec->Areg);
#endif

	Chigh = (byte4)((codec->Creg>>16)&0xffff);
	codec->Areg -= Qe;
	if(Chigh < Qe ){
		if (codec->Areg >= Qe) {
			codec->Areg = Qe;															// D1　LPS
			Di = MPS_D ^ 0x80 ;
			codec->index[ CX ] = (uchar)(QeIndexTable[index]>>24);
			codec->index[ CX ] ^= MPS_D;
		}
		else {
			codec->Areg = Qe;															// D5  MPS
			Di = MPS_D;
			codec->index[ CX ] = (uchar)(((QeIndexTable[index]>>16)&0xff) | MPS_D);
		}
	}
	else{
		codec->Creg -= (ubyte4)((Qe<<16)&0xffff0000);
		if( !(codec->Areg&0x8000) ){
			if (codec->Areg < Qe){
				Di = MPS_D ^ 0x80;														// D2　LPS
				codec->index[ CX ] = (uchar)(((QeIndexTable[index]>>24)&0xff)^MPS_D);
			}
			else{
				Di = MPS_D;																// D4 MPS
				codec->index[ CX ] = (uchar)( ((QeIndexTable[index]>>16)&0xff) | MPS_D);
			}
		}
		else	Di = MPS_D;																// D3 MPS 
	}

	//Renorm
	for( ;!(codec->Areg&0x8000) ; codec->Areg<<= 1){
		if(codec->ctreg==0)
			MQ_ByteIn(codec, str, Eaddr);
		codec->Creg  <<= 1;
		codec->ctreg--;
	}
#if 0
	if(DebugF)
		printf("%d\n",(Di==0? 0:1) );
#endif

	return (Di==0? 0:1);
}

void MQ_setbits(struct mqcodec_s *codec)
{
	byte8 tmp;

	tmp = codec->Creg + codec->Areg;
	codec->Creg |= 0xffff;
	if (codec->Creg >= (unsigned)tmp)
		codec->Creg -= 0x8000;

}

struct StreamChain_s *MQ_flush(struct mqcodec_s *codec, struct StreamChain_s *str)
{
	MQ_setbits(codec);
	codec->Creg <<= codec->ctreg;
	str = MQ_ByteOut(codec, str);		//OutB_buf　Creg(26-CTreg～19-CTreg) go to OutB_buf.
	codec->Creg <<= codec->ctreg;
	str = MQ_ByteOut(codec, str);		//OutB_buf  Creg(18-CTreg～11-CTreg) go to OutB_buf.
	if (codec->B_buf != 0xff){
		codec->Creg <<= codec->ctreg;
		str = MQ_ByteOut(codec, str);
	}
	codec->Creg <<= codec->ctreg;
	str = MQ_ByteOut(codec, str);
		
	return	str;
}

struct StreamChain_s *MQ_ByteOut(struct mqcodec_s *codec, struct StreamChain_s *str)
{
	char	FF_flag;

	if(!codec->first_flag){
		codec->B_buf = (uchar)((codec->Creg) >> 19);
		codec->Creg &= 0x7ffff;
		codec->ctreg = 8;
		codec->first_flag=1;
		return	str;
	}

	if(str->cur_p == str->buf_length){
		if( NULL == (str=StreamChainMake(str, str->buf_length, str->stream_type)) ){
			printf("[mqenc_byteout::StreamChainMake] str create error\n");
			return	NULL;
		}
	}

	if (codec->B_buf == 0xff)	FF_flag=1;
	else{
		codec->B_buf += (codec->Creg & 0x8000000)? 1:0;
		codec->Creg &= 0x7ffffff;
		if (codec->B_buf == 0xff)
			FF_flag=1;	//
		else							FF_flag=0;
		
	}

	str->buf[str->cur_p]=codec->B_buf;
	str->cur_p++;
	str->total_p++;
	if(FF_flag){
		codec->B_buf = (uchar)(codec->Creg >> 20);
		codec->Creg &= 0xfffff;
		codec->ctreg = 7;
	}
	else{
		codec->B_buf = (uchar)((codec->Creg) >> 19);
		codec->Creg &= 0x7ffff;
		codec->ctreg = 8;
	}
	return	str;
}

void MQ_ByteIn(struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Eaddr)
{
	if(str->cur_p < Eaddr   ){
		if (codec->B_buf == 0xff) {
			if(str->buf[str->cur_p]>0x8f){
				codec->ctreg = 8;
				str->cur_p++;
			}
			else{
				codec->B_buf = str->buf[str->cur_p];
				codec->Creg += (byte4)(codec->B_buf<<9);
				codec->ctreg = 7;
				str->cur_p++;
			}
		}
		else{
			codec->B_buf = str->buf[str->cur_p];
			codec->Creg += (byte4)(codec->B_buf<<8) ;
			codec->ctreg = 8;
			str->cur_p++;
		}
	}
	else{
		codec->Creg += 0xff00;
		codec->ctreg = 8;
	}
}


