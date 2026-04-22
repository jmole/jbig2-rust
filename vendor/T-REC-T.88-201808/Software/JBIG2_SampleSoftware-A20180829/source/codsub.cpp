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
#include	"T4T6codec.h"
#include	"ImageUtil.h"

struct StreamChain_s *T4T6Encmain( struct StreamChain_s *str, struct Image_s *Image, byte4 k_para, uchar coding, char RTC2_on )
{
    struct	T4T6Codec_s *codec;
	struct	Huff_Table_s *Huff;
	byte4	width, height, jjj;
	byte4	result;

	width = Image->tbx1-Image->tbx0;
	height = Image->tby1-Image->tby0;

	codec = new struct T4T6Codec_s;
	CodInit(width, k_para, coding, codec);
	Huff = HuffCreate(ENC);

	for(jjj=0; jjj<height ; jjj++){
		result = getlin( codec, Image, jjj );
		str = CodLine( codec, str, Huff );
	}
	if(RTC2_on)
		str = CodEnd(codec, str, Huff );
	str = ByteStuffOutJXR(str);
	return str;
}


struct StreamChain_s *CodLine( struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	switch(c->mode) { 
	case MH:
		str = Cwr_code( EOL, str, Huff );
		str = Cdim1( c->width, c->lbuf, str, Huff );
		break;
	case MR:
		if(c->codkcnt == c->codk) {
			str = Cwr_code(EOL1, str, Huff);
			str = Cdim1( c->width, c->lbuf, str, Huff );
		}
		else {
			str = Cwr_code(EOL0, /*c->cod, c->fpout,*/ str, Huff);
			str = Cdim2( c->width, c->lbuf, c->rbuf, str, Huff );
		}
		c->codkcnt--;
		if(c->codkcnt == 0)
			c->codkcnt = c->codk;
		memcpy(c->rbuf, c->lbuf, c->width );
		break;
	case T6:                         /* T6*/
		str = Cdim2( c->width, c->lbuf, c->rbuf, str, Huff );
		memcpy(c->rbuf, c->lbuf, c->width );
		break;
	default:
		break;
	}
	c->ln++;
	return str;
}


void CodInit( byte4 width, byte4 k_para, uchar coding, struct T4T6Codec_s *codec)
{
	codec->width = width;
	codec->codk  = k_para;
	codec->mode  = coding;
	codec->lbuf  = new uchar [codec->width+2];
	codec->rbuf  = new uchar [codec->width+2];
	codec->xwidth = ceil2(width, 8)*8;
	codec->ln = 0;
	codec->codkcnt = codec->codk;
	memset(codec->rbuf, 0, codec->width);
}

struct Huff_Table_s *HuffCreate(uchar ENCDEC)
{
	struct Huff_Table_s *Huff;
	byte4	iii, kkk;
	byte4	numD, CodeLength;
	Huff = new struct Huff_Table_s;

	if(ENCDEC==ENC){
		Huff->WCodeV = NULL;
		Huff->WCodeC = NULL;
		Huff->WCodeL = NULL;
		Huff->BCodeV = NULL;
		Huff->BCodeC = NULL;
		Huff->BCodeL = NULL;
		Huff->CCodeV = NULL;
		Huff->CCodeC = NULL;
		Huff->CCodeL = NULL;
		Huff->CodeV = new byte4 [ BOFF*2 ];
		Huff->CodeC = new byte4 [ BOFF*2 ];
		Huff->CodeL = new byte4 [ BOFF*2 ];
		memset(Huff->CodeV, 0, sizeof(byte4)*BOFF*2 );
		memset(Huff->CodeC, 0, sizeof(byte4)*BOFF*2 );
		memset(Huff->CodeL, 0, sizeof(byte4)*BOFF*2 );
		//White
		for(iii=0 ; iii<NumTerminateCode+NumMakeUpCode ; iii++){
			Huff->CodeV[iii] = iii;
			Huff->CodeC[iii] = WhiteTerminateCode[iii][1];
			Huff->CodeL[iii] = WhiteTerminateCode[iii][2];
		}
		//Control
		for(iii=0 ; iii<NumControlCode ; iii++){
			Huff->CodeV[PASS+iii] = ControlCode[iii][0];
			Huff->CodeC[PASS+iii] = ControlCode[iii][1];
			Huff->CodeL[PASS+iii] = ControlCode[iii][2];
		}
		//Black
		for(iii=0 ; iii<NumTerminateCode+NumMakeUpCode ; iii++){
			Huff->CodeV[BOFF+iii] = (iii+BOFF);
			Huff->CodeC[BOFF+iii] = BlackTerminateCode[iii][1];
			Huff->CodeL[BOFF+iii] = BlackTerminateCode[iii][2];
		}
	}
	else{
		Huff->WCodeV = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->WCodeC = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->WCodeL = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->BCodeV = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->BCodeC = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->BCodeL = new byte4 [NumTerminateCode+NumMakeUpCode];
		Huff->CCodeV = new byte4 [NumControlCode];
		Huff->CCodeC = new byte4 [NumControlCode];
		Huff->CCodeL = new byte4 [NumControlCode];
		memset(Huff->WCodeV, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->WCodeC, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->WCodeL, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->BCodeV, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->BCodeC, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->BCodeL, 0, sizeof(byte4)*NumTerminateCode+NumMakeUpCode );
		memset(Huff->CCodeV, 0, sizeof(byte4)*NumControlCode );
		memset(Huff->CCodeC, 0, sizeof(byte4)*NumControlCode );
		memset(Huff->CCodeL, 0, sizeof(byte4)*NumControlCode );
		Huff->CodeV = NULL;
		Huff->CodeC = NULL;
		Huff->CodeL = NULL;

		numD = NumTerminateCode+NumMakeUpCode;
		//White
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==WhiteTerminateCode[kkk][2]){
					Huff->WCodeV[iii]    = WhiteTerminateCode[kkk][0];
					Huff->WCodeC[iii]    = WhiteTerminateCode[kkk][1];
					Huff->WCodeL[iii]    = WhiteTerminateCode[kkk][2];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Black
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==BlackTerminateCode[kkk][2]){
					Huff->BCodeV[iii]    = BlackTerminateCode[kkk][0];
					Huff->BCodeC[iii]    = BlackTerminateCode[kkk][1];
					Huff->BCodeL[iii]    = BlackTerminateCode[kkk][2];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Control
		numD = NumControlCode;
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==ControlCode[kkk][2]){
					Huff->CCodeV[iii]    = ControlCode[kkk][0];
					Huff->CCodeC[iii]    = ControlCode[kkk][1];
					Huff->CCodeL[iii]    = ControlCode[kkk][2];
					iii++;
				}
			}
			CodeLength++;			
		}

	}

	return	Huff;
}

struct StreamChain_s *CodEnd( struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4	i;
	switch(c->mode) {
	case MH:
		for(i = 0; i < 6; i++)
			str = Cwr_code( EOL, /*c->cod, c->fpout,*/ str, Huff );
		str = Cwr_code(PAD, str, Huff);
		break;
	case MR:
		for(i = 0; i < 6; i++)
			str = Cwr_code( EOL1, /*c->cod, c->fpout,*/ str, Huff );
		str = Cwr_code(PAD, str, Huff );
		break;
	case T6:
		for(i = 0; i < 2; i++)
			str = Cwr_code( EOL, str, Huff );
		str = Cwr_code( PAD, str, Huff );
		break;
	default:
		break;
	}
	return str;
}


struct StreamChain_s *Cdim1( byte4 dots, uchar *lbuf, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	int worb, dotptr, length;
	worb = 0;
	dotptr = 0;
	length = 0;
	while( dotptr<dots ){
		if( lbuf[dotptr] != worb ){
			str = Cmh( length, worb, /*cod, fpout,*/ str, Huff );
			length = 0;
			worb = worb ^ 1;
		}
		else{
			dotptr++;
			length++;
		}
	}
	str = Cmh(length, worb, /*cod, fpout,*/ str, Huff );
	return str;
}

struct StreamChain_s *Cmh( int length, int worb, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	int llno;
	while(length > 2560) {
		str = Cwr_code(103 + worb * BOFF, str, Huff );
		length = length - 2560;
	}
	if(length >= 64) {
		llno = length / 64 + 63;
		str = Cwr_code(llno + worb * BOFF, str, Huff );
	}
	llno = length %  64;
	str = Cwr_code(llno + worb * BOFF, str, Huff );
	return	str;
}

struct StreamChain_s *Cwr_code( byte4 code_no, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	str = StreamBitWrite( Huff->CodeC[code_no], Huff->CodeL[code_no], str, str->buf_length);
	return str;
}

byte4	Cdetchg( byte4 ptrmax, byte4 sptr, byte4 sworb, uchar *buf )
{
	byte4	dptr;
	dptr = sptr;
	while(1) {
		if(dptr < 0) {
			if(sworb != 0)
				break;
		}
		else if(sworb != buf[dptr])
			break;
		dptr++;
		if(dptr >= ptrmax)
			return(ptrmax);
	}
	while(1) {
		if(dptr < 0) {
			if(sworb == 0)
				break;
		}
		else if(sworb == buf[dptr])
			break;
		dptr++;
		if(dptr >= ptrmax)
			return(ptrmax);
	}
	return(dptr);
}

void	Cdetab( byte4 ptrmax, byte4 *a0, byte4 *a1, byte4 *a2, byte4 *b1, byte4 *b2, uchar *lbuf, uchar *rbuf )
{
	byte4	sworb;
    if(*a0 >= 0)
        sworb = 1 ^ /*lbuf[*a0]*/get_olddat( *a0, lbuf );
    else
        sworb = 1;
    if( *a1<0 )
        *a1 = Cdetchg( ptrmax, *a0, sworb, lbuf );
    if( *b1<0 )
        *b1 = Cdetchg( ptrmax, *a0, sworb, rbuf );

    sworb=1 ^ sworb;
    if(*a1 >= ptrmax)
        *a2 = ptrmax;
    else if(*a2 < 0)
        *a2 = Cdetchg(ptrmax, *a1, sworb, lbuf);
    if(*b1 >= ptrmax)
        *b2 = ptrmax;
    else if(*b2 < 0)
        *b2 = Cdetchg(ptrmax, *b1, sworb, rbuf);
}

struct StreamChain_s *Cdim2( byte4 width, uchar *lbuf, uchar *rbuf, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4 a0, a1, a2, b1, b2, worb, length;
	
	a0 = a1 = a2 = b1 = b2 = -1;
	while( a0<width ){
		Cdetab( width, &a0, &a1, &a2, &b1, &b2, lbuf, rbuf );
		if(a1 > b2) {
			str = Cwr_code( PASS, str, Huff );
			a0 = b2;
			b1 = b2 = -1;
		}
		else if(abs(a1 - b1) <= 3) {
			switch(a1 - b1 + 3) {
			case 0:		str = Cwr_code(VL3, str, Huff );	break;
			case 1:		str = Cwr_code(VL2, str, Huff );	break;
			case 2:		str = Cwr_code(VL1, str, Huff );	break;
			case 3:		str = Cwr_code(V0,  str, Huff );	break;
			case 4:		str = Cwr_code(VR1, str, Huff );	break;
			case 5:		str = Cwr_code(VR2, str, Huff );	break;
			case 6:		str = Cwr_code(VR3, str, Huff );	break;
			}
			a0 = a1;
			if(b2 > a1) {
				if(b1 <= a1) 
					b1 = b2;
				else if(rbuf[a1] != lbuf[a1])
					b1 = b2;
				else
					b1 = -1;
			}
			else
				b1 = -1;
			a1 = a2;
			a2 = b2 = -1;
		}
		else{
			str = Cwr_code( HORZ, str, Huff );
			if(a0 >= 0) {
				worb = lbuf[a0];
				length = a1 - a0;
			}
			else {
				worb = 0;
				length = a1 - a0 - 1;
			}
			str = Cmh(length, worb, str, Huff );
			worb = worb ^ 1;
			length = a2 - a1;
			str = Cmh(length, worb, str, Huff );
			a0 = a2;
			if(a2 >= b1)
				b1 = b2 = -1;
			a1 = a2 = -1;
		}
	}
	return str;
}

byte4 getlin( struct T4T6Codec_s *codec, struct Image_s *Image, byte4 jjj)
{
	int i;
	uchar	*D_TS, *D_;

	D_TS = (uchar *)Image->data;
	D_ = &D_TS[jjj*Image->col1step];
	for(i = 0; i < codec->width ; i++)
		codec->lbuf[i] = D_[i];
	return(0);
}


#if 0
void Cinit_white( char *cod )
{
    strcpy(cod + 16 * 0,"00110101");
    strcpy(cod + 16 * 1,"000111");
    strcpy(cod + 16 * 2,"0111");
    strcpy(cod + 16 * 3,"1000");
    strcpy(cod + 16 * 4,"1011");
    strcpy(cod + 16 * 5,"1100");
    strcpy(cod + 16 * 6,"1110");
    strcpy(cod + 16 * 7,"1111");
    strcpy(cod + 16 * 8,"10011");
    strcpy(cod + 16 * 9,"10100");
    strcpy(cod + 16 * 10,"00111");
    strcpy(cod + 16 * 11,"01000");
    strcpy(cod + 16 * 12,"001000");
    strcpy(cod + 16 * 13,"000011");
    strcpy(cod + 16 * 14,"110100");
    strcpy(cod + 16 * 15,"110101");
    strcpy(cod + 16 * 16,"101010");
    strcpy(cod + 16 * 17,"101011");
    strcpy(cod + 16 * 18,"0100111");
    strcpy(cod + 16 * 19,"0001100");
    strcpy(cod + 16 * 20,"0001000");
    strcpy(cod + 16 * 21,"0010111");
    strcpy(cod + 16 * 22,"0000011");
    strcpy(cod + 16 * 23,"0000100");
    strcpy(cod + 16 * 24,"0101000");
    strcpy(cod + 16 * 25,"0101011");
    strcpy(cod + 16 * 26,"0010011");
    strcpy(cod + 16 * 27,"0100100");
    strcpy(cod + 16 * 28,"0011000");
    strcpy(cod + 16 * 29,"00000010");
    strcpy(cod + 16 * 30,"00000011");
    strcpy(cod + 16 * 31,"00011010");
    strcpy(cod + 16 * 32,"00011011");
    strcpy(cod + 16 * 33,"00010010");
    strcpy(cod + 16 * 34,"00010011");
    strcpy(cod + 16 * 35,"00010100");
    strcpy(cod + 16 * 36,"00010101");
    strcpy(cod + 16 * 37,"00010110");
    strcpy(cod + 16 * 38,"00010111");
    strcpy(cod + 16 * 39,"00101000");
    strcpy(cod + 16 * 40,"00101001");
    strcpy(cod + 16 * 41,"00101010");
    strcpy(cod + 16 * 42,"00101011");
    strcpy(cod + 16 * 43,"00101100");
    strcpy(cod + 16 * 44,"00101101");
    strcpy(cod + 16 * 45,"00000100");
    strcpy(cod + 16 * 46,"00000101");
    strcpy(cod + 16 * 47,"00001010");
    strcpy(cod + 16 * 48,"00001011");
    strcpy(cod + 16 * 49,"01010010");
    strcpy(cod + 16 * 50,"01010011");
    strcpy(cod + 16 * 51,"01010100");
    strcpy(cod + 16 * 52,"01010101");
    strcpy(cod + 16 * 53,"00100100");
    strcpy(cod + 16 * 54,"00100101");
    strcpy(cod + 16 * 55,"01011000");
    strcpy(cod + 16 * 56,"01011001");
    strcpy(cod + 16 * 57,"01011010");
    strcpy(cod + 16 * 58,"01011011");
    strcpy(cod + 16 * 59,"01001010");
    strcpy(cod + 16 * 60,"01001011");
    strcpy(cod + 16 * 61,"00110010");
    strcpy(cod + 16 * 62,"00110011");
    strcpy(cod + 16 * 63,"00110100");
    strcpy(cod + 16 * 64,"11011");
    strcpy(cod + 16 * 65,"10010");
    strcpy(cod + 16 * 66,"010111");
    strcpy(cod + 16 * 67,"0110111");
    strcpy(cod + 16 * 68,"00110110");
    strcpy(cod + 16 * 69,"00110111");
    strcpy(cod + 16 * 70,"01100100");
    strcpy(cod + 16 * 71,"01100101");
    strcpy(cod + 16 * 72,"01101000");
    strcpy(cod + 16 * 73,"01100111");
    strcpy(cod + 16 * 74,"011001100");
    strcpy(cod + 16 * 75,"011001101");
    strcpy(cod + 16 * 76,"011010010");
    strcpy(cod + 16 * 77,"011010011");
    strcpy(cod + 16 * 78,"011010100");
    strcpy(cod + 16 * 79,"011010101");
    strcpy(cod + 16 * 80,"011010110");
    strcpy(cod + 16 * 81,"011010111");
    strcpy(cod + 16 * 82,"011011000");
    strcpy(cod + 16 * 83,"011011001");
    strcpy(cod + 16 * 84,"011011010");
    strcpy(cod + 16 * 85,"011011011");
    strcpy(cod + 16 * 86,"010011000");
    strcpy(cod + 16 * 87,"010011001");
    strcpy(cod + 16 * 88,"010011010");
    strcpy(cod + 16 * 89,"011000");
    strcpy(cod + 16 * 90,"010011011");
    strcpy(cod + 16 * 91,"00000001000");
    strcpy(cod + 16 * 92,"00000001100");
    strcpy(cod + 16 * 93,"00000001101");
    strcpy(cod + 16 * 94,"000000010010");
    strcpy(cod + 16 * 95,"000000010011");
    strcpy(cod + 16 * 96,"000000010100");
    strcpy(cod + 16 * 97,"000000010101");
    strcpy(cod + 16 * 98,"000000010110");
    strcpy(cod + 16 * 99,"000000010111");
    strcpy(cod + 16 * 100,"000000011100");
    strcpy(cod + 16 * 101,"000000011101");
    strcpy(cod + 16 * 102,"000000011110");
    strcpy(cod + 16 * 103,"000000011111");
}

void Cinit_black(char cod[])
{
    strcpy(cod + 16 * (BOFF +   0), "0000110111");
    strcpy(cod + 16 * (BOFF +   1), "010");
    strcpy(cod + 16 * (BOFF +   2), "11");
    strcpy(cod + 16 * (BOFF +   3), "10");
    strcpy(cod + 16 * (BOFF +   4), "011");
    strcpy(cod + 16 * (BOFF +   5), "0011");
    strcpy(cod + 16 * (BOFF +   6), "0010");
    strcpy(cod + 16 * (BOFF +   7), "00011");
    strcpy(cod + 16 * (BOFF +   8), "000101");
    strcpy(cod + 16 * (BOFF +   9), "000100");
    strcpy(cod + 16 * (BOFF +  10), "0000100");
    strcpy(cod + 16 * (BOFF +  11), "0000101");
    strcpy(cod + 16 * (BOFF +  12), "0000111");
    strcpy(cod + 16 * (BOFF +  13), "00000100");
    strcpy(cod + 16 * (BOFF +  14), "00000111");
    strcpy(cod + 16 * (BOFF +  15), "000011000");
    strcpy(cod + 16 * (BOFF +  16), "0000010111");
    strcpy(cod + 16 * (BOFF +  17), "0000011000");
    strcpy(cod + 16 * (BOFF +  18), "0000001000");
    strcpy(cod + 16 * (BOFF +  19), "00001100111");
    strcpy(cod + 16 * (BOFF +  20), "00001101000");
    strcpy(cod + 16 * (BOFF +  21), "00001101100");
    strcpy(cod + 16 * (BOFF +  22), "00000110111");
    strcpy(cod + 16 * (BOFF +  23), "00000101000");
    strcpy(cod + 16 * (BOFF +  24), "00000010111");
    strcpy(cod + 16 * (BOFF +  25), "00000011000");
    strcpy(cod + 16 * (BOFF +  26), "000011001010");
    strcpy(cod + 16 * (BOFF +  27), "000011001011");
    strcpy(cod + 16 * (BOFF +  28), "000011001100");
    strcpy(cod + 16 * (BOFF +  29), "000011001101");
    strcpy(cod + 16 * (BOFF +  30), "000001101000");
    strcpy(cod + 16 * (BOFF +  31), "000001101001");
    strcpy(cod + 16 * (BOFF +  32), "000001101010");
    strcpy(cod + 16 * (BOFF +  33), "000001101011");
    strcpy(cod + 16 * (BOFF +  34), "000011010010");
    strcpy(cod + 16 * (BOFF +  35), "000011010011");
    strcpy(cod + 16 * (BOFF +  36), "000011010100");
    strcpy(cod + 16 * (BOFF +  37), "000011010101");
    strcpy(cod + 16 * (BOFF +  38), "000011010110");
    strcpy(cod + 16 * (BOFF +  39), "000011010111");
    strcpy(cod + 16 * (BOFF +  40), "000001101100");
    strcpy(cod + 16 * (BOFF +  41), "000001101101");
    strcpy(cod + 16 * (BOFF +  42), "000011011010");
    strcpy(cod + 16 * (BOFF +  43), "000011011011");
    strcpy(cod + 16 * (BOFF +  44), "000001010100");
    strcpy(cod + 16 * (BOFF +  45), "000001010101");
    strcpy(cod + 16 * (BOFF +  46), "000001010110");
    strcpy(cod + 16 * (BOFF +  47), "000001010111");
    strcpy(cod + 16 * (BOFF +  48), "000001100100");
    strcpy(cod + 16 * (BOFF +  49), "000001100101");
    strcpy(cod + 16 * (BOFF +  50), "000001010010");
    strcpy(cod + 16 * (BOFF +  51), "000001010011");
    strcpy(cod + 16 * (BOFF +  52), "000000100100");
    strcpy(cod + 16 * (BOFF +  53), "000000110111");
    strcpy(cod + 16 * (BOFF +  54), "000000111000");
    strcpy(cod + 16 * (BOFF +  55), "000000100111");
    strcpy(cod + 16 * (BOFF +  56), "000000101000");
    strcpy(cod + 16 * (BOFF +  57), "000001011000");
    strcpy(cod + 16 * (BOFF +  58), "000001011001");
    strcpy(cod + 16 * (BOFF +  59), "000000101011");
    strcpy(cod + 16 * (BOFF +  60), "000000101100");
    strcpy(cod + 16 * (BOFF +  61), "000001011010");
    strcpy(cod + 16 * (BOFF +  62), "000001100110");
    strcpy(cod + 16 * (BOFF +  63), "000001100111");
    strcpy(cod + 16 * (BOFF +  64), "0000001111");
    strcpy(cod + 16 * (BOFF +  65), "000011001000");
    strcpy(cod + 16 * (BOFF +  66), "000011001001");
    strcpy(cod + 16 * (BOFF +  67), "000001011011");
    strcpy(cod + 16 * (BOFF +  68), "000000110011");
    strcpy(cod + 16 * (BOFF +  69), "000000110100");
    strcpy(cod + 16 * (BOFF +  70), "000000110101");
    strcpy(cod + 16 * (BOFF +  71), "0000001101100");
    strcpy(cod + 16 * (BOFF +  72), "0000001101101");
    strcpy(cod + 16 * (BOFF +  73), "0000001001010");
    strcpy(cod + 16 * (BOFF +  74), "0000001001011");
    strcpy(cod + 16 * (BOFF +  75), "0000001001100");
    strcpy(cod + 16 * (BOFF +  76), "0000001001101");
    strcpy(cod + 16 * (BOFF +  77), "0000001110010");
    strcpy(cod + 16 * (BOFF +  78), "0000001110011");
    strcpy(cod + 16 * (BOFF +  79), "0000001110100");
    strcpy(cod + 16 * (BOFF +  80), "0000001110101");
    strcpy(cod + 16 * (BOFF +  81), "0000001110110");
    strcpy(cod + 16 * (BOFF +  82), "0000001110111");
    strcpy(cod + 16 * (BOFF +  83), "0000001010010");
    strcpy(cod + 16 * (BOFF +  84), "0000001010011");
    strcpy(cod + 16 * (BOFF +  85), "0000001010100");
    strcpy(cod + 16 * (BOFF +  86), "0000001010101");
    strcpy(cod + 16 * (BOFF +  87), "0000001011010");
    strcpy(cod + 16 * (BOFF +  88), "0000001011011");
    strcpy(cod + 16 * (BOFF +  89), "0000001100100");
    strcpy(cod + 16 * (BOFF +  90), "0000001100101");
    strcpy(cod + 16 * (BOFF +  91), "00000001000");
    strcpy(cod + 16 * (BOFF +  92), "00000001100");
    strcpy(cod + 16 * (BOFF +  93), "00000001101");
    strcpy(cod + 16 * (BOFF +  94), "000000010010");
    strcpy(cod + 16 * (BOFF +  95), "000000010011");
    strcpy(cod + 16 * (BOFF +  96), "000000010100");
    strcpy(cod + 16 * (BOFF +  97), "000000010101");
    strcpy(cod + 16 * (BOFF +  98), "000000010110");
    strcpy(cod + 16 * (BOFF +  99), "000000010111");
    strcpy(cod + 16 * (BOFF + 100), "000000011100");
    strcpy(cod + 16 * (BOFF + 101), "000000011101");
    strcpy(cod + 16 * (BOFF + 102), "000000011110");
    strcpy(cod + 16 * (BOFF + 103), "000000011111");
}

void Cinit_2dim(char *cod)
{
    strcpy(cod + 16 * PASS, "0001");
    strcpy(cod + 16 * HORZ, "001");
    strcpy(cod + 16 * V0,   "1");
    strcpy(cod + 16 * VR1,  "011");
    strcpy(cod + 16 * VR2,  "000011");
    strcpy(cod + 16 * VR3,  "0000011");
    strcpy(cod + 16 * VL1,  "010");
    strcpy(cod + 16 * VL2,  "000010");
    strcpy(cod + 16 * VL3,  "0000010");
    strcpy(cod + 16 * EOL,  "000000000001");
    strcpy(cod + 16 * EOL0, "0000000000010");
    strcpy(cod + 16 * EOL1, "0000000000011");
    strcpy(cod + 16 * PAD,  "000000000000");
}
#endif

struct StreamChain_s *StreamBitWrite_LSBFirst(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length)
{
	while(BitLength>0){
		str->buf[str->cur_p] |= ( ((data>>(BitLength-1))&1) <<(8-str->bits) );
		str->bits--;
		if(!str->bits){
			str->cur_p++;
			str->total_p++;
			str->bits=8;
			if(str->cur_p==str->buf_length){
				str = StreamChainMake(str, buf_length, str->stream_type);
			}
		}
		BitLength--;
	}
	return str;
}
