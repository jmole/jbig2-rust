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



#ifndef MQ_CODEC_H
#define MQ_CODEC_H

/********************************************************************************************************/
/********************************************************************************************************/
/************************static const *******************************************************************/
/********************************************************************************************************/
/********************************************************************************************************/
#define J2K   0x01
//#define JBIG2 0x00
#define MQENC_PTERM			  1
#define MQENC_DEFTERM1        2
#define MQENC_DEFTERM2        4
#define MQENC_JBIG2_PROCEDURE 3

static ubyte4 QeIndexTable[47] = {
				//index	//nlps	//nmps	//Qe	//swich
0x81015601,	//	0			1		1	5601	1
0x06023401,//	1			6		2	3401	
0x09031801,//	2			9		3	1801
0x0C040AC1,//	3			C		4	0AC1
0x1D050521,//	4			1D		5	0521	
0x21260221,//	5			21		26	0221
0x86075601,//	6			6		7	5601	1
0x0E085401,//	7
0x0E094801,//	8
0x0E0A3801,//	9
0x110B3001,//	10
0x120C2401,//	11
0x140D1C01,//	12
0x151D1601,//	13
0x8E0F5601,//	14
0x0E105401,//	15
0x0F115101,//	16
0x10124801,//	17
0x11133801,//	18
0x12143401,//	19
0x13153001,//	20
0x13162801,//	21
0x14172401,//	22
0x15182201,//	23
0x16191C01,//	24
0x171A1801,//	25
0x181B1601,//	26
0x191C1401,//	27
0x1A1D1201,//	28
0x1B1E1101,//	29
0x1C1F0AC1,//	30
0x1D2009C1,//	31
0x1E2108A1,//	32
0x1F220521,//	33
0x20230441,//	34
0x212402A1,//	35
0x22250221,//	36
0x23260141,//	37
0x24270111,//	38
0x25280085,//	39
0x26290049,//	40
0x272A0025,//	41
0x282B0015,//	42
0x292C0009,//	43
0x2A2D0005,//	44
0x2B2D0001,//	45
0x2E2E5601 //	46
};

/********************************************************************************************************/
/********************************************************************************************************/
/************************structer**********************************************************************/
/********************************************************************************************************/
/********************************************************************************************************/
typedef struct mqcodec_s{
	ubyte4	Creg;		//0 The C register.
	uchar	*CX;		//8 The Context. for both encoder and decoder
	uchar	*index;		//12 The state index.
	byte4	Areg;		//16 The A register. 
	uchar	ctreg;		//18 The CT register. for both encoder and decoder
	uchar	B_buf;		//19 The byte buffer (i.e., the B variable in the standard). for only encoder
	char	first_flag;	//20
	byte4	numCX;
} mqcodec_t;

void InitMQ_Codec( struct mqcodec_s *codec, struct StreamChain_s *str, byte4 numCX, char dec, byte4 Eaddr, uchar KindOfCode );
struct StreamChain_s *Enc_MQ(struct StreamChain_s *str, struct	mqcodec_s *codec, uchar Di, byte4 CX, char DebugF );
uchar Dec_MQ(struct StreamChain_s *str, struct mqcodec_s *codec, byte4 CX, byte4 Eaddr, char DebugF );
void MQ_ByteIn(struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Eaddr);
struct StreamChain_s *MQ_ByteOut(struct mqcodec_s *codec, struct StreamChain_s *str);
void MQ_DecRenorm(struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Eaddr);
void MQ_setbits(struct mqcodec_s *codec);
struct StreamChain_s *MQ_flush(struct mqcodec_s *codec, struct StreamChain_s *str);
#endif

