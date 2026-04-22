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



struct	Image_s *T4T6DecMain( struct Image_s *Image, struct StreamChain_s *str, uchar coding, uchar lsbmsbFirst, byte4 width, byte4 height )
{
	byte4	jjj;
	byte4	result;
	uchar	*D_TS, *D_;
	struct	T4T6Codec_s *codec;
	struct	Huff_Table_s *Huff;
	struct	ImageChain_s* ImageD, *Child, *Parent;

	codec = new struct T4T6Codec_s;
	Huff = HuffCreate( DEC );
	DecInit(width, height, coding, lsbmsbFirst, codec);

	if( (coding==MH) || (coding==MR) ) {
		DFindEOL( codec, str, Huff );
	}

	if(height==-1){
		ImageD=NULL;
		result = 0;
		Image = NULL;
		while(result!=RTC){
			DecLine( codec, str, Huff );	/*‚Pline decode*/
			ImageD = Put_LineData( ImageD, lsbmsbFirst, codec );
			result = CheckRTC( codec, str, Huff, lsbmsbFirst );
		}
		Ref_StuffBits(str, NoDiscard );	

		Image = ImageCreate(NULL, codec->width, codec->ln, 0, codec->width, 0, codec->height, CHAR);
		D_TS = (uchar *)Image->data;
		while(ImageD!=NULL)
			Parent = ImageD->parent;
		while(ImageD!=NULL)
			Child = ImageD->child;
		ImageD = Parent;
		for(jjj=0 ; jjj <codec->ln ; jjj++, D_TS+=Image->col1step){
			D_ = D_TS;
			memcpy( D_, ImageD->Image->data, sizeof(uchar)*codec->width);
			ImageD = ImageD->child;
		}
		return	Image;
	}
	else{
		Image = ImageCreate(NULL, codec->width, codec->height, 0, codec->width, 0, codec->height, CHAR);
		result = 0;
		for(jjj=0 ; jjj<codec->height ; jjj++ ){
#if T4T6_DEBUG01
			printf("<%d>",jjj);
#endif
			DecLine( codec, str, Huff );
			Put_LineData2( Image, jjj, codec );
		}
		if( RTC==CheckRTC( codec, str, Huff, lsbmsbFirst )){
			if(RTC==CheckRTC( codec, str, Huff, lsbmsbFirst )){
				Ref_StuffBits(str, NoDiscard);	
			}
			else
				Ref_StuffBits(str, NoDiscard);	
		}
		else
			Ref_StuffBits(str, NoDiscard);	
	
		return	Image;
	}
}
uchar get_olddat( byte4 p, uchar *b)
{
	if(p<0)	return	0;
	else	return	b[p];
}

struct ImageChain_s *Put_LineData( struct ImageChain_s *Parent, uchar mode, struct T4T6Codec_s *codec )
{
	byte4	i;
	byte4	bdata = 0;
	uchar	*D_;
	struct ImageChain_s *ImageC;	

	ImageC = ImageChainCreate( Parent );
	ImageC->Image = ImageCreate( NULL, codec->width, 1, 0, codec->width, 0, 1, BIT1);
	if(mode==LSBFirst){
		D_ = (uchar *)ImageC->Image->data;
		for (i = 0; i < codec->width; i++) {
			bdata += codec->lbuf[i] << (i & 0x7);
			if ((i & 0x7) == 7) {
				D_[i/8] = (uchar)bdata;
				bdata = 0;
			}
		}
		if((i & 0x7) != 0) 
			ImageC->Image->data[i/8] = bdata;
	}
	else if(mode==MSBFirst){
	}
	return ImageC;
}

void	Put_LineData2( struct Image_s *Image, byte4 jjj, struct T4T6Codec_s *codec )
{
	byte4	i;
	byte4	bdata = 0;
	uchar	*D_TS, *D_;

	D_TS = (uchar *)Image->data;
	D_   = &D_TS[jjj*Image->col1step];
	for (i = 0; i < codec->width; i++) {
		D_[i] = codec->lbuf[i];
	}
}

void DecInit( byte4 width, byte4 height, uchar coding, uchar lsbmsbFirst, struct T4T6Codec_s *codec )
{
	codec->width = width;
	codec->height = height;
	codec->lsbmsbFirst=lsbmsbFirst;
	codec->mode = coding;
	codec->lbuf = new uchar [codec->width+2];
	codec->rbuf = new uchar [codec->width+2];
	if(coding == T6)
		codec->ddim = 0;
	else
		codec->ddim = 1;
	codec->xwidth = ceil2(codec->width, 8);
	codec->ln = 0;
	memset(codec->rbuf, 0, codec->width);
}

byte4	DecLine( struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff) 
{
	byte4	result;

    memset(codec->lbuf, 0, codec->width);
	switch( codec->mode ){
	case MH:	/*MH*/
		result = DecMHLine( codec, str, Huff );
		break;
	case MR:	/*MR*/
		if (codec->ddim == 1)
			result = DecMHLine( codec, str, Huff );
		else
			result = DecMRLine( codec, str, Huff );
		break;
	case T6:	/*T6*/
		result = DecMRLine( codec, str, Huff );
		break;
	}

	if(result==EOL){
		codec->ln++;
		if(codec->mode != MH)
			memcpy(codec->rbuf, codec->lbuf, codec->width);
	}
	return(result);
}

byte4 DecMRLine( struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4	i, j;
	uchar	color = 0;
	char	tempD;
	byte4	a0, b1, b2;
	byte4	MRC, run;

	codec->count = 0;
	a0 = -1;
	while(codec->count < codec->width) {
		DDetchg(color, &a0, &b1, &b2, codec);
		//MRCode
		if(codec->lsbmsbFirst==LSBFirst)
			MRC = Dec_Huff_LSBFirst(Huff->CCodeV, Huff->CCodeC, Huff->CCodeL, str );
		else
			MRC = Dec_Huff(Huff->CCodeV, Huff->CCodeC, Huff->CCodeL, str );
		switch(MRC) {
		case PASS:
			DSetColor( color, b2, codec );
#if T4T6_DEBUG01
			printf(",[PASS a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			break;
		case HORZ:
#if T4T6_DEBUG01
			printf(",[HORZ a0=%d b1=%d b2=%d],",a0,b1,b2);
#endif
			for(j = 0; j < 2; j++) {
				run = DecMHCode( color, codec, str, Huff );
				if(run>63)
					run+=DecMHCode( color, codec, str, Huff );
				for(i = 0; i < run; i++, codec->count++) {
					codec->lbuf[ codec->count ] = color;
					if(codec->count >= BUFMAX)
						break;
				}
				color ^= 1;
#if T4T6_DEBUG01
				printf("run(%d)=,%d,",j,run);
#endif
			}
#if T4T6_DEBUG01
			printf("\n");
#endif
			break;
		case V0: 
#if T4T6_DEBUG01
			printf(",[V0 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VL1:
#if T4T6_DEBUG01
			printf(",[VL1 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VL2:
#if T4T6_DEBUG01
			printf(",[VL2 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VL3:
#if T4T6_DEBUG01
			printf(",[VL3 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VR1:
#if T4T6_DEBUG01
			printf(",[VR1 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VR2:
#if T4T6_DEBUG01
			printf(",[VR2 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		case VR3:
#if T4T6_DEBUG01
			printf(",[VR3 a0=%d b1=%d b2=%d]\n",a0,b1,b2);
#endif
			tempD = (MRC - V0);
			b1= b1 + (MRC - V0);
			DSetColor(color, b1, codec);
			color ^= 1;
			break;
		}
		a0=codec->count;
	}
	if(codec->mode==MR)
		return	CheckFill_EOL( codec, str, Huff );
	else	//T6
		return	EOL;
}

void DDetchg( char color, byte4 *a0, byte4 *b1, byte4 *b2, struct T4T6Codec_s *codec)
{
	byte4	sptr;
	uchar	D;

	sptr = *a0;
	while( (D=get_olddat(sptr, codec->rbuf)) != color) { 
		sptr++;
		if (sptr >= codec->width) {
			*b1 = codec->width;
			*b2 = codec->width;
			return;
		}
	}
	while( (D=get_olddat(sptr, codec->rbuf)) == color) {
		sptr++;
		if (sptr >= codec->width) {
			*b1 = codec->width;
			*b2 = codec->width;
			return;
		}
	}
	*b1 = sptr;

	while( (D=get_olddat(sptr, codec->rbuf)) != color) {
		sptr++;
		if (sptr >= codec->width) {
			*b2 = codec->width;
			return;
		}
	}
	*b2 = sptr;

}


byte4 DecMHLine( struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	char	color = 0;
	codec->count = 0;
	return	DecMHRun( color, codec, str, Huff );
}

byte4 DecMHRun( char color, struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4	i;
	byte4	run;
    while( codec->count < codec->width ){
		run = DecMHCode( color, codec, str, Huff ); //terminate and makeup both 
		if(run>63){
			//terminate after makeup
			run += DecMHCode( color, codec, str, Huff );
		}
		for(i = 0; i < run; i++, codec->count++) {
			codec->lbuf[ codec->count ] = color;
			if(codec->count >= BUFMAX)
				break;
		}
		color ^=1;
    }
	return	CheckFill_EOL( codec, str, Huff );
}

byte4	CheckFill_EOL(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff  )
{
	byte4	Data;
	byte4	ZeroCnt = 0;

	if(codec->lsbmsbFirst==LSBFirst){
		while(1) {
			Data = Ref_1Bit_LSBFirst(str);
			if(Data)
				break;
			ZeroCnt++;
		}
		if(ZeroCnt >= 11) {
			if(codec->mode == MR)//MR
				codec->ddim = Ref_1Bit_LSBFirst(str);
			return	EOL;
		}
		else
			return	FALES;
	}
	else{
		while(1) {
			Data = Ref_1Bit(str);
			if(Data)
				break;
			ZeroCnt++;
		}
		//EOLi"0000 0000 0001")
		//ZeroCnt>11 FILLB
		if(ZeroCnt >= 11) {
			if(codec->mode == MR)//MR
				codec->ddim = Ref_1Bit(str);
			return	EOL;
		}
		else
			return	FALES;

	}
}

byte4	CheckRTC(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff, uchar lsbmsbFirst )
{
	byte4	Data0;
	byte4	Cur_p, Bits;

	Cur_p = str->cur_p;
	Bits  = str->bits;
	if(lsbmsbFirst==LSBFirst)
		Data0 = Ref_nBits_LSBFirst(str, 12);
	else
		Data0 = Ref_nBits(str, 12);

	if(Data0==1){
		return	RTC;
	}
	else{
		str->cur_p = Cur_p;
		str->bits  = Bits;
		return	TRUE;
	}


}

void DSetColor( char color, byte4 dptr, struct T4T6Codec_s *codec)
{
	while (codec->count < dptr) {
		if (codec->count >= codec->width)
			break;
		codec->lbuf[codec->count] = color;
		codec->count++;
	}
}

byte4	DecMHCode( char color, struct T4T6Codec_s *c, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4	run;

	if( (color==0) && (c->lsbmsbFirst==LSBFirst) )/*white*/
		run = Dec_Huff_LSBFirst( Huff->WCodeV, Huff->WCodeC, Huff->WCodeL, str );
	else if( (color==0) && (c->lsbmsbFirst==MSBFirst) )/*white*/
		run = Dec_Huff( Huff->WCodeV, Huff->WCodeC, Huff->WCodeL, str );
	else if( (color) &&	(c->lsbmsbFirst==LSBFirst) )/*black*/
		run = Dec_Huff_LSBFirst( Huff->BCodeV, Huff->BCodeC, Huff->BCodeL, str );
	else
		run = Dec_Huff( Huff->BCodeV, Huff->BCodeC, Huff->BCodeL, str );
	return run;
}

byte4 DFindEOL(struct T4T6Codec_s *codec, struct StreamChain_s *str, struct Huff_Table_s *Huff )
{
	byte4	eolcnt = 0;
	byte4	Data;

	eolcnt = 0;
	while(1) {
		Data = Ref_1Bit_LSBFirst(str);
		if(Data)
			break;
		eolcnt++;
	}


	if(codec->mode == MR)
		Data=Ref_1Bit_LSBFirst(str);

	return	0;
}

char Ref_1Bit_LSBFirst(struct StreamChain_s *str)
{
	char	data;
	if( str->cur_p >= str->buf_length )	return NULL;

	str->bits--;
	data = str->buf[str->cur_p]&mask4[str->bits] ?	1:0;
	if(str->bits==0){
		str->bits=8;
		str->cur_p++;
	}
	return	data;

}

byte4 Ref_nBits_LSBFirst(struct StreamChain_s *str, uchar bits)
{
	byte4	n=0;
	if(bits){
		n = Ref_1Bit_LSBFirst(str);
		bits--;
		for(;bits>0;bits--){
			n<<=1;
			n += Ref_1Bit_LSBFirst(str);
		}
		return n;
	}
	return FALES;
}

byte4 Dec_Huff_LSBFirst( byte4 *V, byte4 *C, byte4 *L, struct StreamChain_s *str )
{
	byte4	kkk, data, length_K, length_Last;
	byte4	D;
	data=0;
	length_Last=0;
	kkk=-1;
	do{
		kkk++;
		length_K = L[kkk]-length_Last;
		if(length_K){
			D = Ref_nBits_LSBFirst(str, length_K);
			data <<=length_K;
			data += D;
		}
		length_Last = L[kkk];
	} while( data!=C[kkk] );
	return	V[kkk];
}

byte4 Dec_Huff( byte4 *V, byte4 *C, byte4 *L, struct StreamChain_s *str )
{
	byte4	kkk, data, length_K, length_Last;
	data=0;
	length_Last=0;
	kkk=-1;
	do{
		kkk++;
		length_K = L[kkk]-length_Last;
		if(length_K)
			data = (byte4 )( (data<<length_K) | Ref_nBits(str, length_K) );
		length_Last = L[kkk];
	} while( data!=C[kkk] );
	return	V[kkk];
}
