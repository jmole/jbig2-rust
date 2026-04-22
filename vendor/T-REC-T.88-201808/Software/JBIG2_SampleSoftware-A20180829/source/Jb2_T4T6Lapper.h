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



#ifndef		Jb2_T4T6LAPPER_H
#define		Jb2_T4T6LAPPER_H
#include	"Jb2_T4T6Lapper.h"

static	int OOB = 0x80000000;//(signed)-2147483648;
#define		MinusLIMIT	0x80000001	//-2147483647

static byte4	Huffman_Table_A[5][4]={ 
				{ 0x00,0x02,0x06,0x07},		//	EncC
				{    1,   2,   3,   3},		//	EncC_L
				{    4,   8,  16,  32},		//	EncBits
				{    0,  16, 272, 65808},	//	Val					EncVal
				{    0,   0,   0,   0}		//	EncValFunc
				}; 
static byte4	Huffman_Table_B[5][7]={
				{ 0x00,0x02,0x06,0x0e,0x1e,0x3e,0x3f},	//	EncC 
				{    1,   2,   3,   4,   5,   6,   6},	//	EncC_L
				{    0,   0,   0,   3,   6,  32,   0},	//	EncBits
				{    0,   1,   2,   3,  11,  75, OOB},	//	EncVal
				{    0,   0,   0,   0,   0,   0,   0}	//	EncValFunc
				}; 
static byte4	Huffman_Table_C[5][9]={
				{ 0xff,0xfe,0x00,0x02,0x06,0x0e,0x1e,0x7e,0x3e},//
				{    8,   8,   1,   2,   3,   4,   5,   7,   6},//
				{   32,   8,   0,   0,   0,   3,   6,  32,   0},//
				{ -257,-256,   0,   1,   2,   3,  11,  75, OOB},//Val
				{    2,   0,   0,   0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_D[5][6]={ 
				{ 0x00,0x02,0x06,0x0e,0x1e,0x1f},//
				{    1,   2,   3,   4,   5,   5},//
				{    0,   0,   0,   3,   6,  32},//
				{    1,   2,   3,   4,  12,  76},//
				{    0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_E[5][8]={ 
				{ 0x7f,0x7e,0x00,0x02,0x06,0x0e,0x1e,0x3e},//
				{    7,   7,   1,   2,   3,   4,   5,   6},//
				{   32,   8,   0,   0,   0,   3,   6,  32},//
				{ -256,-255,   1,   2,   3,   4,  12,  76},//
				{    2,   0,   0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_F[5][14]={ 
				{ 0x3e, 0x1c, 0x08, 0x09, 0x0a, 0x1d, 0x1e, 0x0b, 0x00, 0x02, 0x03, 0x0c, 0x0d, 0x3f},//
				{    6,    5,    4,    4,    4,    5,    5,    4,    2,    3,    3,    4,    4,    6},//
				{   32,   10,    9,    8,    7,    6,    5,    5,    7,    7,    8,    9,   10,   32},//
				{-2049,-2048,-1024, -512, -256, -128,  -64,  -32,    0,  128,  256,  512, 1024, 2048},//
				{    2,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0} //
				}; 
static byte4	Huffman_Table_G[5][15]={ 
				{ 0x1e, 0x08, 0x00, 0x09, 0x1a, 0x1b, 0x0a, 0x0b, 0x1c, 0x1d, 0x0c, 0x01, 0x02, 0x03, 0x1f},//
				{    5,    4,    3,    4,    5,    5,    4,    4,    5,    5,    4,    3,    3,    3,    5},//
				{   32,    9,    8,    7,    6,    5,    5,    5,    5,    6,    7,    8,    9,   10,   32},//
				{-1025,-1024, -512, -256, -128,  -64,  -32,    0,   32,   64,  128,  256,  512, 1024, 2048},//
				{    2,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0} //
				}; 
static byte4	Huffman_Table_H[5][21]={ 
				{ 0x1fe,0x0fc,0x1fc,0x0fd,0x1fd,0x07c,0x00a,0x000,0x01a,0x03a,0x004,0x03b,0x00b,0x00c,0x01b,0x01c,0x03c,0x07d,0x03d,0x1ff,0x001},//
				{     9,    8,    9,    8,    9,    7,    4,    2,    5,    6,    3,    6,    4,    4,    5,    5,    6,    7,    6,    9,    2},//
				{    32,    3,    1,    1,    0,    0,    0,    1,    0,    0,    4,    1,    4,    5,    6,    7,    7,    8,   10,   32,    0},//
				{   -16,  -15,   -7,   -5,   -3,   -2,   -1,    0,    2,    3,    4,   20,   22,   38,   70,  134,  262,  390,  646, 1670,  OOB},//
				{     2,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0} //
				}; 
static byte4	Huffman_Table_I[5][22]={ 
				{ 0x1fe,0x0fc,0x1fc,0x0fd,0x1fd,0x07c,0x00a,0x002,0x003,0x01a,0x03a,0x004,0x03b,0x00b,0x00c,0x01b,0x01c,0x03c,0x07d,0x03d,0x1ff,0x000},//
				{     9,    8,    9,    8,    9,    7,    4,    3,    3,    5,    6,    3,    6,    4,    4,    5,    5,    6,    7,    6,    9,    2},//
				{    32,    4,    2,    2,    1,    1,    1,    1,    1,    1,    1,    5,    2,    5,    6,    7,    8,    8,    9,   11,   32,    0},//
				{   -32,  -31,  -15,  -11,   -7,   -5,   -3,   -1,    1,    3,    5,    7,   39,   43,   75,  139,  267,  523,  779, 1291, 3339,  OOB},//
				{     2,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0} //
				}; 
static byte4	Huffman_Table_J[5][21]={ 
				{ 0x0fe,0x07a,0x0fc,0x07b,0x018,0x000,0x019,0x036,0x07c,0x0fd,0x001,0x01a,0x037,0x038,0x039,0x03a,0x03b,0x03c,0x07d,0x0ff,0x002},//
				{     8,    7,    8,    7,    5,    2,    5,    6,    7,    8,    2,    5,    6,    6,    6,    6,    6,    6,    7,    8,    2},//
				{    32,    4,    0,    0,    0,    2,    0,    0,    0,    0,    6,    5,    5,    6,    7,    8,    9,   10,   11,   32,    0},//
				{   -22,  -21,   -5,   -4,   -3,   -2,    2,    3,    4,    5,    6,   70,  102,  134,  198,  326,  582, 1094, 2118, 4166,  OOB},//
				{     2,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0,    0} //
				}; 
static byte4	Huffman_Table_K[5][13]={ 
				{ 0x00,0x02,0x0c,0x0d,0x1c,0x1d,0x3c,0x7a,0x7b,0x7c,0x7d,0x7e,0x7f},//
				{    1,   2,   4,   4,   5,   5,   6,   7,   7,   7,   7,   7,   7},//
				{    0,   1,   0,   1,   1,   2,   2,   2,   3,   4,   5,   6,  32},//
				{    1,   2,   4,   5,   7,   9,  13,  17,  21,  29,  45,  77, 141},//
				{    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_L[5][13]={ 
				{ 0x00,0x02,0x06,0x1c,0x1d,0x3c,0x7a,0x7b,0x7c,0x7d,0x7e,0xfe,0xff},//
				{    1,   2,   3,   5,   6,   7,   7,   7,   7,   7,   7,   8,   8},//
				{    0,   0,   1,   0,   1,   1,   0,   1,   2,   3,   4,   5,  32},//
				{    1,   2,   3,   5,   6,   8,  10,  11,  13,  17,  25,  41,  73},//
				{    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_M[5][13]={ 
				{ 0x00,0x04,0x0c,0x1c,0x0d,0x05,0x3a,0x3b,0x3c,0x3d,0x3e,0x7e,0x7f},//
				{    1,   3,   4,   5,   4,   3,   6,   6,   6,   6,   6,   7,   7},//
				{    0,   0,   0,   0,   1,   3,   1,   2,   3,   4,   5,   6,  32},//
				{    1,   2,   3,   4,   5,   7,  15,  17,  21,  29,  45,  77, 141},//
				{    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_N[5][5]={ 
				{ 0x04,0x05,0x00,0x06,0x07},//
				{    3,   3,   1,   3,   3},//
				{    0,   0,   0,   0,   0},//
				{   -2,  -1,   0,   1,   2},//
				{    0,   0,   0,   0,   0} //
				}; 
static byte4	Huffman_Table_O[5][13]={ 
				{ 0x7e,0x7c,0x3c,0x1c,0x0c,0x04,0x00,0x05,0x0d,0x1d,0x3d,0x7d,0x7f},//
				{    7,   7,   6,   5,   4,   3,   1,   3,   4,   5,   6,   7,   7},//
				{   32,   4,   2,   1,   0,   0,   0,   0,   0,   1,   2,   4,  32},//
				{  -25, -24,  -8,  -4,  -2,  -1,   0,   1,   2,   3,   5,   9,  25},//
				{    2,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0} //
				}; 

typedef struct Jb2HuffmanTable_s
{
	uchar	numCode;
	byte4	*DecC;
	byte4	*DecC_L;
	byte4	*DecBits;
	byte4	*DecVal;
	byte4	*DecValFunc;
	byte4	*EncC;
	byte4	*EncC_L;
	byte4	*EncBits;
	byte4	*EncVal;
	byte4	*EncValFunc;
} Jb2HuffmanTable_t;


struct	Jb2HuffmanTable_s *CreateHuffmanTable( char encdec );
byte4	JBIG2_HuffDec( struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff);
struct	StreamChain_s *JBIG2_HuffEnc( byte4 Val, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff);
byte4	JBIG2_ID_Dec( byte4 *C, byte4 *L, byte4 *O, struct StreamChain_s *str );


#endif

