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




#ifndef T4T6codec_h
#define T4T6codec_h

#include	"ImageUtil.h"

#define	T4T6_DEBUG01	0

#define NO_INFO     0
#define NORM_INFO   1
#define DETAIL_INFO 2

#define MH	0
#define MR	1
#define T6	2

#define BUFMAX 8192

//#define CBUFMAX 8192
#define BOFF 128                /*  */
#define PASS 110                /* pass       */
#define HORZ 111                /* hor       */
#define VL3  112                /* VL3             */
#define VL2  113                /* VL2             */
#define VL1  114                /* VL1             */
#define V0   115                /* V0             */
#define VR1  116                /* VR1             */
#define VR2  117                /* VR2             */
#define VR3  118                /* VR3             */
#define EOL  119                /* EOL */
#define EOL0 120                /* EOL  */
#define EOL1 121                /* EOL  */
#define	ZERO6 122				/* DUMY*/
#define PAD  123                /*  */
#define	EOLp1 124
#define	RTC	  125				/**/	


#define	NumTerminateCode 64
#define	NumMakeUpCode    40
#define	NumControlCode   14//15

static byte4 WhiteTerminateCode[NumTerminateCode+NumMakeUpCode][3]={
	{   0,	0x35,	8},//0
	{   1,	0x07,	6},//1
	{   2,	0x07,	4},
	{   3,	0x08,	4},
	{   4,	0x0B,	4},
	{   5,	0x0C,	4},
	{   6,	0x0E,	4},
	{   7,	0x0F,	4},
	{   8,	0x13,	5},
	{   9,	0x14,	5},
	{  10,	0x07,	5},
	{  11,	0x08,	5},
	{  12,	0x08,	6},
	{  13,	0x03,	6},
	{  14,	0x34,	6},
	{  15,	0x35,	6},
	{  16,	0x2A,	6},
	{  17,	0x2B,	6},
	{  18,	0x27,	7},
	{  19,	0x0C,	7},
	{  20,	0x08,	7},
	{  21,	0x17,	7},
	{  22,	0x03,	7},
	{  23,	0x04,	7},
	{  24,	0x28,	7},
	{  25,	0x2B,	7},
	{  26,	0x13,	7},
	{  27,	0x24,	7},
	{  28,	0x18,	7},
	{  29,	0x02,	8},
	{  30,	0x03,	8},
	{  31,	0x1A,	8},
	{  32,	0x1B,	8},
	{  33,	0x12,	8},
	{  34,	0x13,	8},
	{  35,	0x14,	8},
	{  36,	0x15,	8},
	{  37,	0x16,	8},
	{  38,	0x17,	8},
	{  39,	0x28,	8},
	{  40,	0x29,	8},
	{  41,	0x2A,	8},
	{  42,	0x2B,	8},
	{  43,	0x2C,	8},
	{  44,	0x2D,	8},
	{  45,	0x04,	8},
	{  46,	0x05,	8},
	{  47,	0x0A,	8},
	{  48,	0x0B,	8},
	{  49,	0x52,	8},
	{  50,	0x53,	8},
	{  51,	0x54,	8},
	{  52,	0x55,	8},
	{  53,	0x24,	8},
	{  54,	0x25,	8},
	{  55,	0x58,	8},
	{  56,	0x59,	8},
	{  57,	0x5A,	8},
	{  58,	0x5B,	8},
	{  59,	0x4A,	8},
	{  60,	0x4B,	8},
	{  61,	0x32,	8},
	{  62,	0x33,	8},
	{  63,	0x34,	8},
	{  64,	0x1B,	5},//64
	{ 128,	0x12,	5},//65
	{ 192,	0x17,	6},//66
	{ 256,	0x37,	7},//67
	{ 320,	0x36,	8},//68
	{ 384,	0x37,	8},//69
	{ 448,	0x64,	8},//70
	{ 512,	0x65,	8},//71
	{ 576,	0x68,	8},//72
	{ 640,	0x67,	8},//73
	{ 704,	0xCC,	9},//74
	{ 768,	0xCD,	9},//75
	{ 832,	0xD2,	9},//76
	{ 896,	0xD3,	9},//77
	{ 960,	0xD4,	9},//78
	{1024,	0xD5,	9},//79
	{1088,	0xD6,	9},//80
	{1152,	0xD7,	9},//81
	{1216,	0xD8,	9},//82
	{1280,	0xD9,	9},//83
	{1344,	0xDA,	9},//84
	{1408,	0xDB,	9},//85
	{1472,	0x98,	9},//86
	{1536,	0x99,	9},//87
	{1600,	0x9A,	9},//88
	{1664,	0x18,	6},//89
	{1728,	0x9B,	9},//90
	{1792,	0x08,	12},//91
	{1856,	0x0C,	11},//92
	{1920,	0x0D,	11},//93
	{1984,	0x12,	11},//94
	{2048,	0x13,	12},//95
	{2112,	0x14,	12},//96
	{2176,	0x15,	12},//97
	{2240,	0x16,	12},//98
	{2304,	0x17,	12},//99
	{2368,	0x1C,	12},//100
	{2432,	0x1D,	12},//101
	{2496,	0x1E,	12},//102
	{2560,	0x1F,	12}//103
};

static byte4 ControlCode[NumControlCode][3]={
{	PASS,	1,	4	},//110
{	HORZ,	1,	3	},//111
{	VL3,	2,	7	},//112
{	VL2,	2,	6	},//113
{	VL1,	2,	3	},//114
{	V0,		1,	1	},//115
{	VR1,	3,	3	},//116
{	VR2,	3,	6	},//117
{	VR3,	3,	7	},//118
{	EOL,	1,	12	},//119
{	EOL0,	2,	13	},//120
{	EOL1,	3,	13	},//121
{	ZERO6,  0,	6   },//122  Dummy 
{	 PAD,	0,	12	}/*,//123
{	EOLp1, 1,	6	}*/};//124 ZERO6+EOLp1=EOL


static byte4 BlackTerminateCode[NumTerminateCode+NumMakeUpCode][3]={
{	0,	0x37,	10	},//128
{	1,	0x02,	3	},//129
{	2,	0x03,	2	},//130
{	3,	0x02,	2	},//131
{	4,	0x03,	3	},//132
{	5,	0x03,	4	},//133
{	6,	0x02,	4	},//134
{	7,	0x03,	5	},//135
{	8,	0x05,	6	},//136
{	9,	0x04,	6	},//137
{	10,	0x04,	7	},//138
{	11,	0x05,	7	},//139
{	12,	0x07,	7	},//140
{	13,	0x04,	8	},//141
{	14,	0x07,	8	},//142
{	15,	0x18,	9	},//143
{	16,	0x17,	10	},//144
{	17,	0x18,	10	},
{	18,	0x08,	10	},
{	19,	0x67,	11	},
{	20,	0x68,	11	},
{	21,	0x6C,	11	},
{	22,	0x37,	11	},//150
{	23,	0x28,	11	},
{	24,	0x17,	11	},
{	25,	0x18,	11	},
{	26,	0xCA,	12	},
{	27,	0xCB,	12	},
{	28,	0xCC,	12	},
{	29,	0xCD,	12	},
{	30,	0x68,	12	},
{	31,	0x69,	12	},
{	32,	0x6A,	12	},//160
{	33,	0x6B,	12	},
{	34,	0xD2,	12	},
{	35,	0xD3,	12	},
{	36,	0xD4,	12	},
{	37,	0xD5,	12	},
{	38,	0xD6,	12	},
{	39,	0xD7,	12	},
{	40,	0x6C,	12	},
{	41,	0x6D,	12	},
{	42,	0xDA,	12	},//170
{	43,	0xDB,	12	},
{	44,	0x54,	12	},
{	45,	0x55,	12	},
{	46,	0x56,	12	},
{	47,	0x57,	12	},
{	48,	0x64,	12	},
{	49,	0x65,	12	},
{	50,	0x52,	12	},
{	51,	0x53,	12	},
{	52,	0x24,	12	},//180
{	53,	0x37,	12	},
{	54,	0x38,	12	},
{	55,	0x27,	12	},
{	56,	0x28,	12	},
{	57,	0x58,	12	},
{	58,	0x59,	12	},
{	59,	0x2B,	12	},
{	60,	0x2C,	12	},
{	61,	0x5A,	12	},
{	62,	0x66,	12	},//190
{	63,	0x67,	12	},//191
{	64,		0x0F,	10	},//192
{	128,	0xC8,	12	},//193
{	192,	0xC9,	12	},//194
{	256,	0x5B,	12	},
{	320,	0x33,	12	},
{	384,	0x34,	12	},
{	448,	0x35,	12	},
{	512,	0x6C,	13	},
{	576,	0x6D,	13	},//200
{	640,	0x4A,	13	},
{	704,	0x4B,	13	},
{	768,	0x4C,	13	},
{	832,	0x4D,	13	},
{	896,	0x72,	13	},
{	960,	0x73,	13	},
{	1024,	0x74,	13	},
{	1088,	0x75,	13	},
{	1152,	0x76,	13	},
{	1216,	0x77,	13	},//210
{	1280,	0x52,	13	},
{	1344,	0x53,	13	},
{	1408,	0x54,	13	},
{	1472,	0x55,	13	},
{	1536,	0x5A,	13	},
{	1600,	0x5B,	13	},
{	1664,	0x64,	13	},
{	1728,	0x65,	13	},
{	1792,	0x08,	13	},
{	1856,	0x0C,	13	},//210
{	1920,	0x0D,	13	},
{	1984,	0x12,	13	},
{	2048,	0x13,	13	},
{	2112,	0x14,	13	},
{	2176,	0x15,	13	},
{	2240,	0x16,	13	},
{	2304,	0x17,	13	},
{	2368,	0x1C,	13	},
{	2432,	0x1D,	13	},
{	2496,	0x1E,	13	},//220
{	2560,	0x1F,	13	}};//221

typedef struct T4T6Codec_s{
	byte4	width;
	byte4	xwidth;
	byte4	height;
    byte4	ln;
	byte4	count;
	uchar	*lbuf;
	uchar	*rbuf;
	uchar	mode;
	uchar	ddim;
	uchar	codk; 
	uchar	codkcnt;
	uchar	lsbmsbFirst;
} T4T6Codec_t;

typedef	struct	Huff_Table_s{
	byte4	*CodeV;
	byte4	*CodeC;
	byte4	*CodeL;
	byte4	*WCodeV;
	byte4	*WCodeC;
	byte4	*WCodeL;
	byte4	*BCodeV;
	byte4	*BCodeC;
	byte4	*BCodeL;
	byte4	*CCodeV;
	byte4	*CCodeC;
	byte4	*CCodeL;
} Huff_Table_t;


struct	Image_s	*T4T6DecMain( struct Image_s *Image, struct StreamChain_s *str, uchar coding, uchar lsbmsbFirst, byte4 width, byte4 height);

struct	ImageChain_s *Put_LineData( struct ImageChain_s *Parent, uchar mode, struct T4T6Codec_s *codec );
void	Put_LineData2( struct Image_s *Image, byte4 jjj, struct T4T6Codec_s *codec );
struct	StreamChain_s *T4T6Encmain(struct StreamChain_s *str, struct Image_s *Image, byte4 k_para, uchar coding, char RTC2_on);
void	CodInit( byte4 width, byte4 k_para, uchar coding, struct T4T6Codec_s *c);
struct	StreamChain_s *CodLine(struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff);
struct	StreamChain_s *CodEnd(struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff);
void	Cinit_white(char *cod);	
void	Cinit_black(char *cod);
void	Cinit_2dim(char *cod);
struct	StreamChain_s *Cwr_code(int code_no, /*char *cod, FILE *fpout,*/ struct StreamChain_s *str, struct Huff_Table_s *Huff );
struct	StreamChain_s *Cdim1( byte4 dots, uchar *lbuf, struct StreamChain_s *str, struct Huff_Table_s *Huff );
struct	StreamChain_s *Cmh(int length, int worb, /*char *cod, FILE *fpout,*/ struct StreamChain_s *str, struct Huff_Table_s *Huff );
byte4	Cdetchg(int ptrmax, int sptr, int sworb, uchar *buf);
void	Cdetab( byte4 ptrmax, int *a0, int *a1, int *a2, int *b1, int *b2, uchar *lbuf, uchar *rbuf );
struct	StreamChain_s *Cdim2( byte4 dots, uchar *lbuf, uchar *rbuf, struct StreamChain_s *str, struct Huff_Table_s *Huff );
byte4	getlin( struct T4T6Codec_s *c, struct Image_s *Image, byte4 jjj); 
void	para_errEnc(void);
byte4	CheckFill_EOL(struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff  );
byte4	CheckRTC(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff, uchar lsbmsbFirst );

uchar get_olddat( byte4 p, uchar *b);
struct Huff_Table_s *HuffCreate(uchar ENCDEC);

struct StreamChain_s *StreamBitWrite_LSBFirst(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);

int  DecLine(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff ); 
int  DFindEOL(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );
void DecInit( byte4 width, byte4 height, uchar coding, uchar lsbmsbFirst, struct T4T6Codec_s *codec );
void para_errDec(void);
void Deof_msg(struct T4T6Codec_s *codec);
void Derr_msg(struct T4T6Codec_s *codec);
void Derr_msg1(struct T4T6Codec_s *codec);

void Dinitm(struct T4T6Codec_s *codec);                /* mr             */
void Dinitw(struct T4T6Codec_s *codec);                /* mh white*/
void Dinitb(struct T4T6Codec_s *codec);                /* mh black */
void DPrtPoint(struct T4T6Codec_s *codec);             /*  */
int  DCheckEOL(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );             /* EOL check*/
//int  DCheckRTC(int mode, struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );   /* RTC check*/
void DSetColor( char color, byte4 dptr, struct T4T6Codec_s *codec);
byte4	DecMHCode( char color, struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff ); /* mh white */
byte4	DecMHRun(  char color, struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );   /* MH  */
byte4	DecMHLine(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );             /* MH */
//int  DecMRCode(int *code, struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );  /* mr */
void DDetchg( char color, byte4 *a0, byte4 *b1, byte4 *b2, struct T4T6Codec_s *codec);      /*  */
int  DecMRLine(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff );             /* MR */

char Ref_1Bit_LSBFirst(struct StreamChain_s *s);
byte4 Ref_nBits_LSBFirst(struct StreamChain_s *s, uchar bits);
byte4 Dec_Huff_LSBFirst( byte4 *V, byte4 *C, byte4 *L, struct StreamChain_s *str );
byte4 Dec_Huff( byte4 *V, byte4 *C, byte4 *L, struct StreamChain_s *str );

#endif
