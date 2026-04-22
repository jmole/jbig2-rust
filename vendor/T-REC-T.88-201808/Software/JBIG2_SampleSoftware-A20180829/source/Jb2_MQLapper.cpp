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
#include	"ImageUtil.h"
#include	"Jb2Common.h"
#include	"Jb2_MQLapper.h"
#include	"Jb2_Debug.h"
#include	"Jb2_T4T6Lapper.h"



void InitMQ_Codec2( struct mqcodec_s *codec )
{
	memset( codec->index, 0, sizeof(uchar)*codec->numCX );
}

byte4 CX_RefEncode( byte4 i, byte4 ref_i, byte4 width, byte4 ref_width, uchar *rD2_, uchar *rD1_, uchar *rD0_, uchar *D1_, uchar *D0_, uchar rTemplate, uchar *A1_, uchar *rA2_, char ATX1, char rATX2 )
{
	byte4	CX;
	uchar	B10,B9,B8,B7,B6,B5,B4,B3,B2,B1,B0,A2,A1;

	switch(rTemplate){
	case 0:

		B10  = Dout1( D1_, (i  ), width );
		B9  = Dout1( D1_, (i+1), width );

		B8  = Dout1( D0_, (i-1), width );

		B7  = Dout1( rD2_, (ref_i  ), ref_width );
		B6  = Dout1( rD2_, (ref_i+1), ref_width );

		B5  = Dout1( rD1_, (ref_i-1), ref_width );
		B4  = Dout1( rD1_, (ref_i  ), ref_width );
		B3  = Dout1( rD1_, (ref_i+1), ref_width );

		B2  = Dout1( rD2_, (ref_i-1), ref_width );
		B1  = Dout1( rD2_, (ref_i  ), ref_width );
		B0  = Dout1( rD2_, (ref_i+1), ref_width );

		A1  = Dout1( A1_, (i+ATX1), width );
		A2  = Dout1( rA2_, (ref_i+rATX2), ref_width );

		CX = (A2<<12) + (A1<<11) + (B10<<10) + (B9<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
		break;
	case 1:

		B9  = Dout1( D1_, (i-1), width );
		B8  = Dout1( D1_, (i  ), width );
		B7  = Dout1( D1_, (i+1), width );
		B6  = Dout1( D0_, (i-1), width );
		B5  = Dout1( rD2_, (ref_i  ), ref_width );
		B4  = Dout1( rD1_, (ref_i-1), ref_width );
		B3  = Dout1( rD1_, (ref_i  ), ref_width );
		B2  = Dout1( rD1_, (ref_i+1), ref_width );
		B1  = Dout1( rD0_, (ref_i  ), ref_width );
		B0  = Dout1( rD0_, (ref_i+1), ref_width );

		CX = (B9<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
		break;
	}
	return	CX;
}

byte4 CX_Encode( byte4 j, byte4 i, byte4 width,  struct Image_s *Image, uchar *D2_, uchar *D1_, uchar *D0_, uchar Template, uchar ExtTemplate, char ATX1, char ATX2, char ATX3, char ATX4, char ATX5, char ATX6, char ATX7, char ATX8, char ATX9, char ATX10, char ATX11, char ATX12, uchar *A1_, uchar *A2_, uchar *A3_, uchar *A4_, uchar *A5_, uchar *A6_, uchar *A7_, uchar *A8_, uchar *A9_, uchar *A10_, uchar *A11_, uchar *A12_)
{
	byte4	CX;
	byte4	col1step;
	uchar	B11,B10,B9,B8,B7,B6,B5,B4,B3,B2,B1,B0,A12,A11,A10,A9,A8,A7,A6,A5,A4,A3,A2,A1;

	col1step = Image->col1step;

	if(!ExtTemplate){
		switch(Template){
		case 0:
			B11 = Dout1( D2_, (i-1), width );
			B10 = Dout1( D2_, (i+0), width );
			B9  = Dout1( D2_, (i+1), width );

			B8  = Dout1( D1_, (i-2), width );
			B7  = Dout1( D1_, (i-1), width );
			B6  = Dout1( D1_, (i  ), width );
			B5  = Dout1( D1_, (i+1), width );
			B4  = Dout1( D1_, (i+2), width );

			B3  = Dout1( D0_, (i-4), width );
			B2  = Dout1( D0_, (i-3), width );
			B1  = Dout1( D0_, (i-2), width );
			B0  = Dout1( D0_, (i-1), width );

			A4  = Dout1( A4_, (i+ATX4), width );
			A3  = Dout1( A3_, (i+ATX3), width );
			A2  = Dout1( A2_, (i+ATX2), width );
			A1  = Dout1( A1_, (i+ATX1), width );

			CX = (A4<<15) + (A3<<14) + (A2<<13) + (A1<<12) + (B11<<11) + (B10<<10) + (B9<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
			break;
		case 1:
			B11 = Dout1( D2_, (i-1), width );
			B10 = Dout1( D2_, (i+0), width );
			B9  = Dout1( D2_, (i+1), width );
			B8  = Dout1( D2_, (i+2), width );

			B7  = Dout1( D1_, (i-2), width );
			B6  = Dout1( D1_, (i-1), width );
			B5  = Dout1( D1_, (i  ), width );
			B4  = Dout1( D1_, (i+1), width );
			B3  = Dout1( D1_, (i+2), width );

			B2  = Dout1( D0_, (i-3), width );
			B1  = Dout1( D0_, (i-2), width );
			B0  = Dout1( D0_, (i-1), width );

			A1  = Dout1( A1_, (i+ATX1), width );

			CX = (A1<<12) + (B11<<11) + (B10<<10) + (B9<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
			break;
		case 2:
			B8 = Dout1( D2_, (i-1), width );
			B7 = Dout1( D2_, (i+0), width );
			B6 = Dout1( D2_, (i+1), width );

			B5 = Dout1( D1_, (i-2), width );
			B4 = Dout1( D1_, (i-1), width );
			B3 = Dout1( D1_, (i  ), width );
			B2 = Dout1( D1_, (i+1), width );

			B1 = Dout1( D0_, (i-2), width );
			B0 = Dout1( D0_, (i-1), width );

			A1 = Dout1( A1_, (i+ATX1), width );

			CX = (A1<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
			break;
		case 3:
			B8  = Dout1( D1_, (i-3), width );
			B7  = Dout1( D1_, (i-2), width );
			B6  = Dout1( D1_, (i-1), width );
			B5  = Dout1( D1_, (i  ), width );
			B4  = Dout1( D1_, (i+1), width );

			B3  = Dout1( D0_, (i-4), width );
			B2  = Dout1( D0_, (i-3), width );
			B1  = Dout1( D0_, (i-2), width );
			B0  = Dout1( D0_, (i-1), width );

			A1  = Dout1( A1_, (i+ATX1), width );

			CX = (A1<<9) + (B8<<8) + (B7<<7) + (B6<<6) + (B5<<5) + (B4<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
			break;
		}
	}
	else{
		B3 = Dout1( D1_, (i-1), width );
		B2 = Dout1( D1_, (i+0), width );
		B1  = Dout1( D1_, (i+1), width );

		B0  = Dout1( D0_, (i-1), width );
		A12  = Dout1( A12_, (i+ATX12), width );
		A11  = Dout1( A11_, (i+ATX11  ), width );
		A10  = Dout1( A10_, (i+ATX10), width );
		A9  = Dout1( A9_, (i+ATX9), width );

		A8  = Dout1( A8_, (i+ATX8), width );
		A7  = Dout1( A7_, (i+ATX7), width );
		A6  = Dout1( A6_, (i+ATX6), width );
		A5  = Dout1( A5_, (i+ATX5), width );

		A4  = Dout1( A4_, (i+ATX4), width );
		A3  = Dout1( A3_, (i+ATX3), width );
		A2  = Dout1( A2_, (i+ATX2), width );
		A1  = Dout1( A1_, (i+ATX1), width );
		CX = (A12<<15) + (A11<<14) + (A10<<13) + (A9<<12) + (A8<<11) + (A7<<10) + (A6<<9) + (A5<<8) + (A4<<7) + (A3<<6) + (A2<<5) + (A1<<4) + (B3<<3) + (B2<<2) + (B1<<1) + (B0);
	}
	return	CX;
}

#if 1
struct StreamChain_s *MQ_RefinementEncImage( struct Image_s *RefImage, struct Image_s *ImageC, byte4 RefDx, byte4 RefDy, struct mqcodec_s *codec, struct StreamChain_s *str, char TpGDon, char rTemplate, char rATX1, char rATY1, char rATX2, char rATY2)
{
	//MQ Dec
	byte4	CX, i,  j, ref_i, ref_j;
	byte4	dec=1, col1step, rcol1step, rwidth, rheight;
	byte4	height, width;
	uchar	*D_TS, *D0_, *D1_, *A1_;			//1:up line 0:myself line (Image)
	uchar	*rD_TS, *rD0_, *rD1_, *rD2_, *rA2_;	//2:up line 1:myself line 0:down line (RefImage)
	uchar	*D_ZERO, *rD_ZERO;
	uchar	LTP;

	height = ImageC->tby1-ImageC->tby0;
	width  = ImageC->tbx1-ImageC->tbx0;
	col1step = ImageC->col1step;
	D_ZERO = new uchar [col1step];
	memset( D_ZERO, 0, sizeof(uchar)*col1step );

	rheight= RefImage->tby1-RefImage->tby0;
	rwidth = RefImage->tbx1-RefImage->tbx0;
	rcol1step = RefImage->col1step;
	rD_ZERO = new uchar	[rcol1step];
	memset( rD_ZERO, 0, sizeof(uchar)*rcol1step );

#if (JBIG2_DEBUG02 || JBIG2_DEBUG06)
	printf("[MQ_RefinementEncImage], height=%d, width=%d, rheight=%d, rwidth=%d, RefDy=%d, RefDx=%d, rTemplate=%d, rATX1=%d, rATY1=%d, rATX2=%d, rATY2=%d\n",height, width, rheight, rwidth, RefDy, RefDx, rTemplate, rATX1, rATY1, rATX2, rATY2 );
#endif

	D_TS  = (uchar *)ImageC->data;
	rD_TS = (uchar *)RefImage->data;
	LTP=0;
	for(j=0, ref_j=j-RefDy ; j<height ; j++, ref_j++ ){
		if(j==0){	D1_ = D_ZERO;	D0_ = D_TS;			 }//Top line
		else{		D1_ = D0_;		D0_ = &D0_[col1step];}

		if( (j+rATY1)<0)			A1_= D_ZERO;
		else if( (j+rATY1)<height)	A1_= &D_TS[(j+rATY1)*col1step];
		else						A1_= D_ZERO;

		if(ref_j<-1){					rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}
		else if(ref_j==-1){				rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j==0){				rD2_ = rD_ZERO;						rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j<(rheight-1)) {	rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j==(rheight-1)){	rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = rD_ZERO;						}
		else if(ref_j==rheight){		rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}
		else{							rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}

		if( (ref_j+rATY2)<0 )				rA2_= rD_ZERO;
		else if( (ref_j+rATY2)<rheight )	rA2_= &rD_TS[(ref_j+rATY2)*rcol1step];
		else								rA2_= rD_ZERO;

		if(TpGDon){
			switch(rTemplate){
			case 0:	CX=0xc395;	break;
			case 1: CX=0x0795;	break;
			case 2:	CX=0x0271;	break;
			default:CX=0x02c5;	break;
			}
			str = Enc_MQ(str, codec, D0_[i], CX, 0 );
			LTP ^= D0_[i];
			if(!LTP){
				for(i=0 ; i<width ; i++ ){
					CX = CX_RefEncode( i, ref_i, width, rwidth, rD2_, rD1_, rD0_, D1_, D0_, rTemplate, A1_, rA2_, rATX1, rATX2 );
					str = Enc_MQ(str, codec, D0_[i], CX, 0 );
				}
			}
			else{
				for(i=0 ; i<width ; i++ )
					D0_[i] = D1_[i];
			}
		}
		else{
			for(i=0, ref_i=i-RefDx ; i<width ; i++, ref_i++ ){
				CX = CX_RefEncode( i, ref_i, width, rwidth, rD2_, rD1_, rD0_, D1_, D0_, rTemplate, A1_, rA2_, rATX1, rATX2 );
				str = Enc_MQ(str, codec, D0_[i], CX, 0 );
			}
		}
	}
	delete	[] D_ZERO;

#if (JBIG2_DEBUG02 || JBIG2_DEBUG06)
	printf("[MQ_RefinementEncImage END]\n");
#endif

	return	str;
}
#endif

struct Image_s *MQ_RefinementDecImage( struct Image_s *RefImage, byte4 width, byte4 height, byte4 RefDx, byte4 RefDy, struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Code_Length, char TpGDon, char rTemplate, char rATX1, char rATY1, char rATX2, char rATY2)
{
	//MQ Dec
	struct	Image_s *ImageC=NULL;
	byte4	CX, i,  j, ref_i, ref_j;
	byte4	dec=1, col1step, rcol1step, rwidth, rheight;
	uchar	*D_TS, *D0_, *D1_, *A1_;			//1:up line 0:myself line (Image)
	uchar	*rD_TS, *rD0_, *rD1_, *rD2_, *rA2_;	//2:up line 1:myself line 0:down line (RefImage)
	uchar	*D_ZERO, *rD_ZERO;
	uchar	Di, LTP;

	ImageC = ImageCreate( ImageC, width, height, 0, width, 0, height, CHAR );
	col1step = ImageC->col1step;
	D_ZERO = new uchar [col1step];
	memset( D_ZERO, 0, sizeof(uchar)*col1step );

	rheight= RefImage->tby1-RefImage->tby0;
	rwidth = RefImage->tbx1-RefImage->tbx0;
	rcol1step = RefImage->col1step;
	rD_ZERO = new uchar	[rcol1step];
	memset( rD_ZERO, 0, sizeof(uchar)*rcol1step );

#if (JBIG2_DEBUG02 || JBIG2_DEBUG06)
	printf("[MQ_RefinementDecImage], height=%d, width=%d, rheight=%d, rwidth=%d, RefDy=%d, RefDx=%d, rTemplate=%d, rATX1=%d, rATY1=%d, rATX2=%d, rATY2=%d\n",height, width, rheight, rwidth, RefDy, RefDx, rTemplate, rATX1, rATY1, rATX2, rATY2 );
#endif

	D_TS  = (uchar *)ImageC->data;
	rD_TS = (uchar *)RefImage->data;
	LTP=0;
	for(j=0, ref_j=j-RefDy ; j<height ; j++, ref_j++ ){
		if(j==0){	D1_ = D_ZERO;	D0_ = D_TS;			 }//Top line
		else{		D1_ = D0_;		D0_ = &D0_[col1step];}

		if( (j+rATY1)<0)			A1_= D_ZERO;
		else if( (j+rATY1)<height)	A1_= &D_TS[(j+rATY1)*col1step];
		else						A1_= D_ZERO;

		if(ref_j<-1){					rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}
		else if(ref_j==-1){				rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j==0){				rD2_ = rD_ZERO;						rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j<(rheight-1)) {	rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = &rD_TS[(ref_j+1)*rcol1step];}
		else if(ref_j==(rheight-1)){	rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = &rD_TS[(ref_j+0)*rcol1step];	rD0_ = rD_ZERO;						}
		else if(ref_j==rheight){		rD2_ = &rD_TS[(ref_j-1)*rcol1step];	rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}
		else{							rD2_ = rD_ZERO;						rD1_ = rD_ZERO;						rD0_ = rD_ZERO;						}

		if( (ref_j+rATY2)<0 )				rA2_= rD_ZERO;
		else if( (ref_j+rATY2)<rheight )	rA2_= &rD_TS[(ref_j+rATY2)*rcol1step];
		else								rA2_= rD_ZERO;

		if(TpGDon){
			switch(rTemplate){
			case 0:	CX=0xc395;	break;
			case 1: CX=0x0795;	break;
			case 2:	CX=0x0271;	break;
			default:CX=0x02c5;	break;
			}
			Di = Dec_MQ( str, codec, CX, Code_Length, 0 );
			LTP ^= Di;
			if(!LTP){
				for(i=0 ; i<width ; i++ ){
					CX = CX_RefEncode( i, ref_i, width, rwidth, rD2_, rD1_, rD0_, D1_, D0_, rTemplate, A1_, rA2_, rATX1, rATX2 );
					Di = Dec_MQ( str, codec, CX, Code_Length, 0 );
					D0_[i] = Di;
				}
			}
			else{
				for(i=0 ; i<width ; i++ )
					D0_[i] = D1_[i];
			}
		}
		else{
			for(i=0, ref_i=i-RefDx ; i<width ; i++, ref_i++ ){
				CX = CX_RefEncode( i, ref_i, width, rwidth, rD2_, rD1_, rD0_, D1_, D0_, rTemplate, A1_, rA2_, rATX1, rATX2 );
				Di = Dec_MQ( str, codec, CX, Code_Length, 0 );
				D0_[i] = Di;

			}
		}
	}
	delete	[] D_ZERO;
#if (JBIG2_DEBUG02 || JBIG2_DEBUG06)
	printf("[MQ_RefinementDecImage End]\n");
#endif
	return	ImageC;
}

struct	StreamChain_s *MQ_EncImage( struct Image_s *Image, struct StreamChain_s *str, struct mqcodec_s *codec, char TpGDon, char Template, char ExtTemplate, char ATX1, char ATY1, char ATX2, char ATY2, char ATX3, char ATY3, char ATX4, char ATY4, char ATX5, char ATY5, char ATX6, char ATY6, char ATX7, char ATY7, char ATX8, char ATY8, char ATX9, char ATY9, char ATX10, char ATY10, char ATX11, char ATY11, char ATX12, char ATY12, char DebugF )
{
	byte4	CX, i, j;
	byte4	enc_dec=0, col1step, height;
	uchar	*DD_, *D0_, *D1_, *D2_, *A12_, *A11_, *A10_, *A9_, *A8_, *A7_, *A6_, *A5_, *A4_, *A3_, *A2_, *A1_;
	uchar	*D_ZERO;

	col1step = Image->col1step;
	height = Image->height;
	D_ZERO = new uchar [col1step];
	memset(D_ZERO, 0, sizeof(uchar)*col1step);

	if(!ExtTemplate){
		ATX5 = 1;	ATY5 =-2;
		ATX6 = 2;	ATY6 =-1;
		ATX7 =-3;	ATY7 = 0;
		ATX8 =-4;	ATY8 = 0;
		ATX9 = 2;	ATY9 =-2;
		ATX10= 3;	ATY10=-1;
		ATX11=-2;	ATY11=-2;
		ATX12=-3;	ATY12=-1;
	}


	//MQ Enc
	if(TpGDon){
	}
	else{
		D0_ = (uchar *)Image->data;
		DD_ = D0_;
		for(j=0 ; j<(Image->tby1-Image->tby0) ; j++ ){
			if(j==0){	D2_ = D_ZERO;	D1_=D_ZERO;						}
			else{		D2_ = D1_;		D1_=D0_;	D0_=&D0_[col1step];	}
			if( (j+ATY1)<0)				A1_= D_ZERO;
			else if((j+ATY1)>=height)	A1_ = D_ZERO;
			else						A1_= &DD_[(j+ATY1)*col1step];
			if( (j+ATY2)<0)				A2_= D_ZERO;
			else if((j+ATY2)>=height)	A2_ = D_ZERO;
			else						A2_= &DD_[(j+ATY2)*col1step];
			if( (j+ATY3)<0)				A3_= D_ZERO;
			else if((j+ATY3)>=height)	A3_ = D_ZERO;
			else						A3_= &DD_[(j+ATY3)*col1step];
			if( (j+ATY4)<0)				A4_= D_ZERO;
			else if((j+ATY4)>=height)	A4_ = D_ZERO;
			else						A4_= &DD_[(j+ATY4)*col1step];
			if( (j+ATY5)<0)				A5_= D_ZERO;
			else if((j+ATY5)>=height)	A5_ = D_ZERO;
			else						A5_= &DD_[(j+ATY5)*col1step];
			if( (j+ATY6)<0)				A6_= D_ZERO;
			else if((j+ATY6)>=height)	A6_ = D_ZERO;
			else						A6_= &DD_[(j+ATY6)*col1step];
			if( (j+ATY7)<0)				A7_= D_ZERO;
			else if((j+ATY7)>=height)	A7_ = D_ZERO;
			else						A7_= &DD_[(j+ATY7)*col1step];
			if( (j+ATY8)<0)				A8_= D_ZERO;
			else if((j+ATY8)>=height)	A8_ = D_ZERO;
			else						A8_= &DD_[(j+ATY8)*col1step];
			if( (j+ATY9)<0)				A9_= D_ZERO;
			else if((j+ATY9)>=height)	A9_ = D_ZERO;
			else						A9_= &DD_[(j+ATY9)*col1step];
			if( (j+ATY10)<0)			A10_= D_ZERO;
			else if((j+ATY10)>=height)	A10_ = D_ZERO;
			else						A10_= &DD_[(j+ATY10)*col1step];
			if( (j+ATY11)<0)			A11_= D_ZERO;
			else if((j+ATY11)>=height)	A11_ = D_ZERO;
			else						A11_= &DD_[(j+ATY11)*col1step];
			if( (j+ATY12)<0)			A12_= D_ZERO;
			else if((j+ATY12)>=height)	A12_ = D_ZERO;
			else						A12_= &DD_[(j+ATY12)*col1step];

			for(i=0 ; i<(Image->tbx1-Image->tbx0) ; i++ ){
				CX = CX_Encode( j, i, Image->width,  Image, D2_, D1_, D0_, Template, ExtTemplate, ATX1, ATX2, ATX3, ATX4, ATX5, ATX6, ATX7, ATX8, ATX9, ATX10, ATX11, ATX12, A1_, A2_, A3_, A4_, A5_, A6_, A7_, A8_, A9_, A10_, A11_, A12_);
#if JBIG2_DEBUG02
				printf("Enc_MQ, %d, %d\n",CX, D0_[i]);
#endif
				str = Enc_MQ(str, codec, D0_[i], CX, DebugF );
			}
		}
	}
	delete	[] D_ZERO;
	return	str;
}

struct Image_s *MQ_DecImage( byte4 width, byte4 height, struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Code_Length, char TpGDon, char Template, char ExtTemplate, char ATX1, char ATY1, char ATX2, char ATY2, char ATX3, char ATY3, char ATX4, char ATY4, char ATX5, char ATY5, char ATX6, char ATY6, char ATX7, char ATY7, char ATX8, char ATY8, char ATX9, char ATY9, char ATX10, char ATY10, char ATX11, char ATY11, char ATX12, char ATY12)
{
	//MQ Dec
	struct	Image_s *Image=NULL;
	byte4	CX, i, j;
	byte4	dec=1, col1step;
	uchar	*DD_, *D_TS, *D0_, *D1_, *D2_, *A12_, *A11_, *A10_, *A9_, *A8_, *A7_, *A6_, *A5_, *A4_, *A3_, *A2_, *A1_;
	uchar	*D_ZERO;
	uchar	Di, LTP;
	char	DebugF=0;

	if(!ExtTemplate){
		ATX5 = 1;	ATY5 =-2;
		ATX6 = 2;	ATY6 =-1;
		ATX7 =-3;	ATY7 = 0;
		ATX8 =-4;	ATY8 = 0;
		ATX9 = 2;	ATY9 =-2;
		ATX10= 3;	ATY10=-1;
		ATX11=-2;	ATY11=-2;
		ATX12=-3;	ATY12=-1;
	}

	Image=ImageCreate(Image, width, height, 0, width, 0, height, CHAR);
	col1step = Image->col1step;
	D_ZERO = new uchar [col1step];
	memset(D_ZERO, 0, sizeof(uchar)*col1step);

	D_TS = (uchar *)Image->data;
	D0_ = D_TS;
	LTP=0;
	if(TpGDon){
		D0_ = (uchar *)Image->data;
		DD_ = D0_;
		for(j=0 ; j<height ; j++ ){
			if(j==0){	D2_ = D_ZERO;	D1_=D_ZERO;						}
			else{		D2_ = D1_;		D1_=D0_;	D0_=&D0_[col1step];	}

			if( (j+ATY1)<0)				A1_= D_ZERO;
			else if((j+ATY1)>=height)	A1_ = D_ZERO;
			else						A1_= &DD_[(j+ATY1)*col1step];
			if( (j+ATY2)<0)				A2_= D_ZERO;
			else if((j+ATY2)>=height)	A2_ = D_ZERO;
			else						A2_= &DD_[(j+ATY2)*col1step];
			if( (j+ATY3)<0)				A3_= D_ZERO;
			else if((j+ATY3)>=height)	A3_ = D_ZERO;
			else						A3_= &DD_[(j+ATY3)*col1step];
			if( (j+ATY4)<0)				A4_= D_ZERO;
			else if((j+ATY4)>=height)	A4_ = D_ZERO;
			else						A4_= &DD_[(j+ATY4)*col1step];
			if( (j+ATY5)<0)				A5_= D_ZERO;
			else if((j+ATY5)>=height)	A5_ = D_ZERO;
			else						A5_= &DD_[(j+ATY5)*col1step];
			if( (j+ATY6)<0)				A6_= D_ZERO;
			else if((j+ATY6)>=height)	A6_ = D_ZERO;
			else						A6_= &DD_[(j+ATY6)*col1step];
			if( (j+ATY7)<0)				A7_= D_ZERO;
			else if((j+ATY7)>=height)	A7_ = D_ZERO;
			else						A7_= &DD_[(j+ATY7)*col1step];
			if( (j+ATY8)<0)				A8_= D_ZERO;
			else if((j+ATY8)>=height)	A8_ = D_ZERO;
			else						A8_= &DD_[(j+ATY8)*col1step];
			if( (j+ATY9)<0)				A9_= D_ZERO;
			else if((j+ATY9)>=height)	A9_ = D_ZERO;
			else						A9_= &DD_[(j+ATY9)*col1step];
			if( (j+ATY10)<0)			A10_= D_ZERO;
			else if((j+ATY10)>=height)	A10_ = D_ZERO;
			else						A10_= &DD_[(j+ATY10)*col1step];
			if( (j+ATY11)<0)			A11_= D_ZERO;
			else if((j+ATY11)>=height)	A11_ = D_ZERO;
			else						A11_= &DD_[(j+ATY11)*col1step];
			if( (j+ATY12)<0)			A12_= D_ZERO;
			else if((j+ATY12)>=height)	A12_ = D_ZERO;
			else						A12_= &DD_[(j+ATY12)*col1step];

			switch(Template){
			case 0:	CX=0xc395;	break;
			case 1: CX=0x0795;	break;
			case 2:	CX=0x0271;	break;
			default:CX=0x02c5;	break;
			}
			Di = Dec_MQ( str, codec, CX, Code_Length, DebugF );
			LTP ^= Di;
			if(!LTP){
				for(i=0 ; i<(Image->tbx1-Image->tbx0) ; i++ ){
					CX = CX_Encode( j, i, Image->width,  Image, D2_, D1_, D0_, Template, ExtTemplate, ATX1, ATX2, ATX3, ATX4, ATX5, ATX6, ATX7, ATX8, ATX9, ATX10, ATX11, ATX12, A1_, A2_, A3_, A4_, A5_, A6_, A7_, A8_, A9_, A10_, A11_, A12_);
					Di = Dec_MQ( str, codec, CX, Code_Length, DebugF );
					D0_[i] = Di;
				}
			}
			else{
				for(i=0 ; i<(Image->tbx1-Image->tbx0) ; i++ )
					D0_[i] = D1_[i];
			}
		}
	}
	else{
		D0_ = (uchar *)Image->data;
		DD_ = D0_;
		for(j=0 ; j<(Image->tby1-Image->tby0) ; j++ ){
			if(j==0){	D2_ = D_ZERO;	D1_=D_ZERO;						}
			else{		D2_ = D1_;		D1_=D0_;	D0_=&D0_[col1step];	}
			if( (j+ATY1)<0)				A1_= D_ZERO;
			else if((j+ATY1)>=height)	A1_ = D_ZERO;
			else						A1_= &DD_[(j+ATY1)*col1step];
			if( (j+ATY2)<0)				A2_= D_ZERO;
			else if((j+ATY2)>=height)	A2_ = D_ZERO;
			else						A2_= &DD_[(j+ATY2)*col1step];
			if( (j+ATY3)<0)				A3_= D_ZERO;
			else if((j+ATY3)>=height)	A3_ = D_ZERO;
			else						A3_= &DD_[(j+ATY3)*col1step];
			if( (j+ATY4)<0)				A4_= D_ZERO;
			else if((j+ATY4)>=height)	A4_ = D_ZERO;
			else						A4_= &DD_[(j+ATY4)*col1step];
			if( (j+ATY5)<0)				A5_= D_ZERO;
			else if((j+ATY5)>=height)	A5_ = D_ZERO;
			else						A5_= &DD_[(j+ATY5)*col1step];
			if( (j+ATY6)<0)				A6_= D_ZERO;
			else if((j+ATY6)>=height)	A6_ = D_ZERO;
			else						A6_= &DD_[(j+ATY6)*col1step];
			if( (j+ATY7)<0)				A7_= D_ZERO;
			else if((j+ATY7)>=height)	A7_ = D_ZERO;
			else						A7_= &DD_[(j+ATY7)*col1step];
			if( (j+ATY8)<0)				A8_= D_ZERO;
			else if((j+ATY8)>=height)	A8_ = D_ZERO;
			else						A8_= &DD_[(j+ATY8)*col1step];
			if( (j+ATY9)<0)				A9_= D_ZERO;
			else if((j+ATY9)>=height)	A9_ = D_ZERO;
			else						A9_= &DD_[(j+ATY9)*col1step];
			if( (j+ATY10)<0)			A10_= D_ZERO;
			else if((j+ATY10)>=height)	A10_ = D_ZERO;
			else						A10_= &DD_[(j+ATY10)*col1step];
			if( (j+ATY11)<0)			A11_= D_ZERO;
			else if((j+ATY11)>=height)	A11_ = D_ZERO;
			else						A11_= &DD_[(j+ATY11)*col1step];
			if( (j+ATY12)<0)			A12_= D_ZERO;
			else if((j+ATY12)>=height)	A12_ = D_ZERO;
			else						A12_= &DD_[(j+ATY12)*col1step];
			for(i=0 ; i<(Image->tbx1-Image->tbx0) ; i++ ){
				CX = CX_Encode( j, i, Image->width,  Image, D2_, D1_, D0_, Template, ExtTemplate, ATX1, ATX2, ATX3, ATX4, ATX5, ATX6, ATX7, ATX8, ATX9, ATX10, ATX11, ATX12, A1_, A2_, A3_, A4_, A5_, A6_, A7_, A8_, A9_, A10_, A11_, A12_);
				Di = Dec_MQ( str, codec, CX, Code_Length, DebugF );
				D0_[i] = Di;
#if JBIG2_DEBUG02
				printf("Dec_MQ,%d,%d\n",CX, Di);
#endif
			}
		}
	}
	delete	[] D_ZERO;
	return	Image;
}

struct StreamChain_s *MQ_EncIntegerIAID( byte4 Val, struct StreamChain_s *str, struct mqcodec_s *codec, byte4 SymbolCodeLength, byte4 bCX )
{
	byte4	nCX, CX, k;
	uchar	Di;
	char	DebugF=0;

#if JBIG2_DEBUG06
	printf("[MQ_IntegerIAID]\n");
#endif

	nCX = 1;
	for( k=0 ; k<SymbolCodeLength ; k++){
		CX = nCX;
		Di = (Val>>(SymbolCodeLength-1-k))&1;
		str  = Enc_MQ( str, codec, Di, CX+bCX, DebugF );
		nCX &= mask2[SymbolCodeLength+1];
		nCX = (nCX<<1) | Di;
	}

#if JBIG2_DEBUG06
	printf("[MQ_IntegerIAID END]\n");
#endif

	return	str;

}

byte4 MQ_DecIntegerIAID( struct StreamChain_s *str, struct mqcodec_s *codec, byte4 Code_Length, byte4 SymbolCodeLength, byte4 bCX )
{
	byte4	nCX, CX, k, V;
	uchar	Di;
	char DebugF=0;

#if JBIG2_DEBUG06
	printf("[MQ_IntegerIAID]\n");
#endif

	nCX = 1;
	for( k=0 ; k<SymbolCodeLength ; k++){
		CX = nCX;
		Di  = Dec_MQ( str, codec, CX+bCX, Code_Length, DebugF );
		nCX &= mask2[SymbolCodeLength+1];
		nCX = (nCX<<1) | Di;
	}
	V = nCX & mask2[SymbolCodeLength];

#if JBIG2_DEBUG06
	printf("[MQ_IntegerIAID END]\n");
#endif	

	return	V;

}

struct StreamChain_s *MQ_EncInteger( byte4 V, struct StreamChain_s *str, struct mqcodec_s *codec, byte4 bCX )
{
	byte4	k;
	byte4	nCX, CX;
	uchar	S, Di;
	char	DebugF = 0;

#if JBIG2_DEBUG06
	printf("[MQ_Integer]\n");
#endif
	if(V==OOB){
		CX=0x01;	S=1;
		str = Enc_MQ( str, codec, S, CX+bCX, DebugF );
		CX=0x03;	Di=0;
		str = Enc_MQ( str, codec, Di, CX+bCX, DebugF );
		CX=0x06;	Di=0;
		str = Enc_MQ( str, codec, Di, CX+bCX, DebugF );
		CX=0x0c;	Di=0;
		str = Enc_MQ( str, codec, Di, CX+bCX, DebugF );
	}
	else if(V<=-4436){
		V = (-V-4036);
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x03;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x07;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x0f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x1f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x3f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		for( k=0, nCX=((CX<<1)|1) ; k<32 ; k++ ){
			CX=nCX;
			Di = (V>>(31-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=-340){
		V = (-V-340);
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x03;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x07;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x0f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x1f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x3f;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF );
		for( k=0, nCX=((CX<<1)|0) ; k<12 ; k++ ){
			CX=nCX;
			Di = (V>>(11-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=-84){
		V = (-V-84);
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x03;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x07;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x0f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF );
		CX=0x1f;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF );
		for( k=0, nCX=((CX<<1)|0) ; k<8 ; k++ ){
			CX=nCX;
			Di = (V>>(7-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=-20){
		V = (-V-20);
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x03;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x07;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x0f;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<6 ; k++ ){
			CX=nCX;
			Di = (V>>(5-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF  );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=-4){//-19<=>-4
		V = (-V-4);
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x03;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x07;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<4 ; k++ ){
			CX=nCX;
			Di = (V>>(3-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF  );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=-1){//-3,-2,-1
		V = -V;
		CX=0x01;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x03;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<2 ; k++ ){
			CX=nCX;
			Di = (V>>(1-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF  );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=3){//0,1,2,3
		V = V;
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<2 ; k++ ){
			CX=nCX;
			Di = (V>>(1-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=19){//4<=>19
		V = (V-4);
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x05;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<4 ; k++ ){
			CX=nCX;
			Di = (V>>(3-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=83){
		V = (V-20);
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x05;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x0b;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<6 ; k++ ){
			CX=nCX;
			Di = (V>>(5-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=339){
		V = (V-84);
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x05;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x0b;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x17;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<8 ; k++ ){
			CX=nCX;
			Di = (V>>(7-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else if(V<=4435){
		V = (V-340);
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x05;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x0b;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x17;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x2f;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|0) ; k<12 ; k++ ){
			CX=nCX;
			Di = (V>>(11-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
	else{
		V = (V-4436);
		CX=0x01;		str = Enc_MQ( str, codec, 0, CX+bCX, DebugF  );
		CX=0x02;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x05;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x0b;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x17;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		CX=0x2f;		str = Enc_MQ( str, codec, 1, CX+bCX, DebugF  );
		for( k=0, nCX=((CX<<1)|1) ; k<32 ; k++ ){
			CX=nCX;
			Di = (V>>(31-k))&1;
			str = Enc_MQ(str, codec, Di, CX+bCX, DebugF );
			(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
		}
	}
#if JBIG2_DEBUG06
	printf("[MQ_Integer END]\n");
#endif
	return	str;
}

byte4	MQ_DecInteger( struct StreamChain_s *str, struct mqcodec_s *codec, byte4 bCX, byte4 Code_Length, uchar *Flag )
{
	//MQ Dec
	byte4	nCX, CX, k;
	byte4	Di, V=0;
	uchar	S;
	char	DebugF=0;

#if JBIG2_DEBUG06
	printf("[MQ_Integer]\n");
#endif

	nCX = 1;
	CX=nCX;
	S  = Dec_MQ( str, codec, CX+bCX, Code_Length, DebugF );
	nCX = (nCX<<1) | S ;
	CX=nCX;
	Di = Dec_MQ( str, codec, CX+bCX, Code_Length, DebugF );
	nCX=((nCX<<1)|Di);
	if( !Di ){
		//"Sing 0xx"
		V  = 0;
		CX=nCX;
		Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
		nCX=((nCX<<1)|Di);
		V=((V<<1)|Di);
		CX=nCX;
		Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
		nCX=((nCX<<1)|Di);
		V=((V<<1)|Di);
		if(S && V==0)
			*Flag=0;
	}
	else{
		//"Sing 1"
		CX=nCX;
		Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
		nCX=((nCX<<1)|Di);
		if( !Di ){
			//"Sign 10xxxx
			V  = 0;
			for(k=0;k<4;k++){
				CX=nCX;
				Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
				nCX=((nCX<<1)|Di);
				V=((V<<1)|Di);
			}
			V += 4;
		}
		else{
			//"Sign 11"
			CX=nCX;
			Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
			nCX=((nCX<<1)|Di);
			if( !Di ){
				//"Sign 1 10xx xxxx"
				V  = 0;
				for(k=0;k<6;k++){
					CX=nCX;
					Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
					(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
					V=((V<<1)|Di);
				}
				V += 20;
			}
			else{
				//"Sign 111"
				CX=nCX;
				Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
				nCX=((nCX<<1)|Di);
				if( !Di ){
					//"Sign 1110 xxxx xxxx"
					V  = 0;
					for(k=0;k<8;k++){
						CX=nCX;
						Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
						(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
						V=((V<<1)|Di);
					}
					V += 84;
				}
				else{
					//"Sign 1111"
					CX=nCX;
					Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
					nCX=((nCX<<1)|Di);
					if( !Di ){
						//"Sign 1 1110 xxxx xxxx xxxx"
						V  = 0;
						for(k=0;k<12;k++){
							CX=nCX;
							Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
							(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
							V=((V<<1)|Di);
						}
						V += 340;
					}
					else{
						//"Sign 1 1111 xxxx xxxx xxxx xxxx xxxx xxxx xxxx xxxx"
						V  = 0;
						for(k=0;k<32;k++){
							CX=nCX;
							Di=Dec_MQ(str, codec, CX+bCX, Code_Length, DebugF);
							(nCX>0xff)? nCX=((((nCX<<1)|Di)&0x1ff)|0x100):nCX=((nCX<<1)|Di);
							V=((V<<1)|Di);
						}
						V += 4436;
					}
				}
			}
		}
	}
#if JBIG2_DEBUG06
	printf("[MQ_Integer END]\n");
#endif
	if(S){
		if(V){
			V *= -1;
			return	V;
		}
		else
			return	OOB;
	}
	else
		return	V;
}


