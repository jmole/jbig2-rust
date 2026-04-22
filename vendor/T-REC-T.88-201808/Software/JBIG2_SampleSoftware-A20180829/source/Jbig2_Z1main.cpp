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
#include	<math.h>
#include	<string.h>
#include	<malloc.h>
#include	"ImageUtil.h"
#include	"Jb2Common.h"
#include	"Jb2_T4T6Lapper.h"
#include	"Jb2_MQLapper.h"

byte4 main(int argc, char **argv)
{
	struct	Jb2HuffmanTable_s *EncHuff;
	struct	Jb2HuffmanTable_s *DecHuff;
	struct	mqcodec_s *codec;
	struct	StreamChain_s *str=NULL;
	struct	Image_s *Image, *rImage, *RefImage;
	struct  Image_s *ImageV, *RefImageV;
	byte4	i, j;
	byte4	Val;
	byte4	numCode=256;
	byte4	Saddr, Sbits;
	byte4	*DDD, *EEE, Flag;
	byte4	bCX=0x10400, numCX=0x12000, *CX;
	byte4	col1step, rcol1step, MQ_Eaddr;
	byte4	width, height, rwidth, rheight;
	uchar	*D_, *rD_;
	uchar	FFlag=0;
	uchar	Template=1, rTemplate=0, TpGDon=0, ExtTemplate=0;
	char	ATX1=3, ATY1=-1, ATX2=-3, ATY2=-1, ATX3=2, ATY3=-2, ATX4=-2, ATY4=-2;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0;
	char	ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=-2;
	char	rATX1=-1, rATY1=0, rATX2=0, rATY2=0, rATX3=0, rATY3=0, rATX4=0, rATY4=0;
	char	RefDy=-2, RefDx=1;
	char	fname1[256], fname2[256];

	DDD = new byte4 [numCode];
	EEE = new byte4 [numCode];
	CX  = new byte4 [numCode];
	EncHuff = CreateHuffmanTable( ENC );
	DecHuff = CreateHuffmanTable( DEC );

	//Table_A
	for(j=0;j<15;j++){
		str = StreamChainMake(str, 0x8000, NoDiscard);
		Flag=0;
		for(i=0;i<numCode;i++){
			Val = rand()*i;
			if(j==0 || j==1){
				if(Val<0)
					Val=0;
			}
			DDD[i] = (byte4)Val;
		}
		if(j==2){
			DDD[0]=-258;
			DDD[1]=-257;
			DDD[2]=-256;
			DDD[3]=-255;
		}
		if( (j==3) || (j==10) || (j==11) || (j==12) ){
			for(i=0;i<numCode;i++){
				if(DDD[i]<=0)
					DDD[i]=1;
			}
		}
		if(j==13){
			for(i=0;i<numCode;i++){
				DDD[i]=i%3;
			}
		}
		Saddr = str->cur_p;
		Sbits = str->bits;
		for(i=0;i<numCode;i++){
			str = JBIG2_HuffEnc( DDD[i], str, &EncHuff[j]);
		}
		str->cur_p=Saddr;
		str->bits=Sbits;
		for(i=0;i<numCode;i++){
			Val = JBIG2_HuffDec( str, &DecHuff[j] );
			EEE[i] = Val;
		}
		for(i=0;i<numCode;i++){
			if(DDD[i]!=EEE[i]){
				printf("Table_(%d) ERROR DDD[%d]=%d EEE[%d]=%d\n",j, i,DDD[i],i,EEE[i]);
				Flag++;
			}
		}
		if(!Flag)
			printf("Table_(%d) is OK\n", j);

		delete	[] str->buf;
		delete	str;
		str = NULL;
	}

	//MQ Test
	for(i=0;i<numCode;i++){
		Val = (rand()*i)%0x8000;
		DDD[i] = Val;
		CX[i] = 0;
	}
	DDD[0]=1;
	DDD[1]=2;
	DDD[2]=0;
	DDD[3]=1;
	DDD[4]=1;
	DDD[5]=0;
	DDD[6]=1;
	DDD[7]=2;
	DDD[8]=0;
	DDD[9]=3;
	DDD[0xa]=0;
	DDD[0xb]=0;
	DDD[0xc]=1;
	DDD[0xd]=2;
	DDD[0xe]=0;
	DDD[0xf]=1;
	DDD[0x10]=1;
	DDD[0x11]=OOB;
	DDD[0x12]=3;
	DDD[0x13]=8;
	DDD[0x14]=5;
	DDD[0x15]=OOB;
	DDD[0x16]=-2;
	DDD[0x17]=6;
	DDD[0x18]=0;
	CX[0]=IADT;
	CX[1]=IADT;
	CX[2]=IAFS;
	CX[3]=IAIT;
	CX[4]=IADW;
	CX[5]=IADS;
	CX[6]=IAIT;
	CX[7]=IADW;
	CX[8]=IADS;
	CX[9]=IAIT;
	CX[0xa]=IADW;
	CX[0xb]=IADS;
	CX[0xc]=IAIT;
	CX[0xd]=IADW;
	CX[0xe]=IADS;
	CX[0xf]=IAIT;
	CX[0x10]=IARI;
	CX[0x11]=IADS;
	CX[0x12]=IADT;
	CX[0x13]=IADH;//8
	CX[0x14]=IADW;//5
	CX[0x15]=IADW;//OOB
	CX[0x16]=IADH;//-2
	CX[0x17]=IADW;//6
	CX[0x18]=IADW;//0
	str = StreamChainMake(str, 0x8000, NoDiscard);
	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec( codec, str, numCX, ENC, str->buf_length, JBIG2 );
	for(i=0;i<0x19;i++)
		str = MQ_EncInteger( DDD[i], str, codec, CX[i] );
	str = MQ_flush( codec, str);
	delete	[] codec->index;
	delete	codec;
	str->cur_p=0;
	str->bits=8;
	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec( codec, str, numCX, DEC, str->buf_length, JBIG2 );
	i=0;
	FFlag=1;
	for(i=0;i<0x19;i++){
		FFlag=1;
		EEE[i] = MQ_DecInteger( str, codec, CX[i], str->buf_length, &FFlag );
		if(!FFlag)
			EEE[i]=OOB;
	}
	for( i=0,Flag=0 ; i<0x19 ; i++ ){
		if(DDD[i]!=EEE[i]){
			printf("MQ_Integer ERROR DDD[%d]=%d EEE[%d]=%d\n", i, DDD[i], i, EEE[i]);
			Flag++;
		}
	}
	if(!Flag)
		printf("MQ_Integer is OK\n");
	delete	[] codec->index;
	delete	codec;

	//MQ (IAID)Test
	for(i=0;i<numCode;i++){
		Val = abs((rand()*i))%4;
		DDD[i] = i%3;
	}
	str = StreamChainMake(str, 0x8000, NoDiscard);
	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec( codec, str, numCX, ENC, str->buf_length, JBIG2 );
	for(i=0;i<numCode;i++){
		str = MQ_EncIntegerIAID( DDD[i], str, codec, 2, bCX );
	}
	str = MQ_flush( codec, str);
	delete	[] codec->index;
	delete	codec;


	str->cur_p=0;
	str->bits=8;
	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec( codec, str, numCX, DEC, str->buf_length, JBIG2 );
	for(i=0;i<numCode;i++){
		EEE[i] = MQ_DecIntegerIAID( str, codec, str->total_p, 2, bCX );
	}

	for( i=0,Flag=0 ; i<numCode ; i++ ){
		if(DDD[i]!=EEE[i]){
			printf("MQ_Integer(IAID) ERROR DDD[%d]=%d EEE[%d]=%d\n", i, DDD[i], i, EEE[i]);
			Flag++;
		}
	}
	if(!Flag)
		printf("MQ_Integer(IAID) is OK\n");
	delete	[] codec->index;
	delete	codec;

	//MQ_EncImage MQ_DecImage
	width = 16;
	height = 16;
	rwidth=width;
	rheight=height;
	Image = ImageCreate(NULL, width,height,0,width,0,height,CHAR);
	RefImage = ImageCreate(NULL, rwidth, rheight, 0, rwidth, 0, rheight, CHAR);
	col1step = Image->col1step;
	rcol1step = RefImage->col1step;
	D_ = (uchar *)Image->data;
	memset(D_, 0, sizeof(uchar)*col1step*height);
	D_[0*col1step+0]=1;
	D_[1*col1step+1]=1;
	D_[2*col1step+2]=1;
	D_[3*col1step+3]=1;
	D_[4*col1step+4]=1;
	D_[5*col1step+5]=1;
	D_[6*col1step+6]=1;
	D_[7*col1step+7]=1;
	D_[8*col1step+8]=1;
	D_[9*col1step+9]=1;
	D_[10*col1step+10]=1;
	D_[11*col1step+11]=1;
	D_[12*col1step+12]=1;
	D_[13*col1step+13]=1;
	D_[14*col1step+14]=1;
	D_[15*col1step+15]=1;
	D_ = (uchar *)RefImage->data;
	memset(D_, 0, sizeof(uchar)*rcol1step*(rheight) );
	D_[0*rcol1step+0]=1;
	D_[1*rcol1step+1]=1;
	D_[2*rcol1step+2]=1;
	D_[3*rcol1step+3]=1;
	D_[4*rcol1step+4]=1;
	D_[5*rcol1step+5]=1;
	D_[6*rcol1step+6]=1;
	D_[7*rcol1step+7]=1;
	D_[8*rcol1step+8]=1;
	D_[9*rcol1step+9]=1;
	D_[10*rcol1step+10]=1;
	D_[11*rcol1step+11]=1;
	D_[12*rcol1step+12]=1;
	D_[13*rcol1step+13]=1;
	D_[14*rcol1step+14]=1;
	D_[15*rcol1step+15]=1;
	memset( str->buf, 0, sizeof(uchar)*str->buf_length);
	str->cur_p=0;
	str->bits=8;

	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
	str = MQ_EncImage( Image, str, codec, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12, 0);
	str = MQ_flush(codec, str);

	MQ_Eaddr = str->cur_p;
	str->cur_p=0;
	InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
	rImage = MQ_DecImage( width, height, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
		   //MQ_DecImage( Width[i], Height, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
	//struct Image_s *MQ_DecImage( byte4 width, byte4 height, struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Code_Length, char TpGDon, char Template, char ExtTemplate, char ATX1, char ATY1, char ATX2, char ATY2, char ATX3, char ATY3, char ATX4, char ATY4, char ATX5, char ATY5, char ATX6, char ATY6, char ATX7, char ATY7, char ATX8, char ATY8, char ATX9, char ATY9, char ATX10, char ATY10, char ATX11, char ATY11, char ATX12, char ATY12)
	rD_ = (uchar *)rImage->data;
	D_ = (uchar *)Image->data;
	Flag=0;
	for(j=0;j<height;j++){
		for(i=0;i<width;i++){
			if( rD_[j*col1step+i] != D_[j*col1step+i] ){
				printf("MQ_EncImage/MQ_DecImage ERROR!! %d %d %x %x\n",j,i,rD_[j*col1step+i], D_[j*col1step+i] );
				Flag=1;
			}
		}
	}
	if(!Flag)
		printf("MQ_EncImage/MQ_DecImage is OK\n");
	delete	[] codec->index;
	delete	codec;
	delete	[] rImage->Pdata;
	delete	rImage;
	delete	[] Image->Pdata;
	delete	Image;


	//MQ_RefinementEncImage / MQ_RefinementDecImage
	strcpy(fname1, "Sym001.bmp");
	strcpy(fname2, "Sym000.bmp");
	RefImageV = (struct Image_s *)LoadBmp(fname1);//C
	RefImage  = (Image_s *)ImageBit1ToChar(RefImageV);
	ImageV = (struct Image_s *)LoadBmp(fname2);//P
	Image     = (Image_s *)ImageBit1ToChar(ImageV);
	codec = new struct mqcodec_s;
	codec->numCX=numCX;
	codec->index = new uchar [codec->numCX];
	memset( &str->buf[0], 0, sizeof(uchar)*str->buf_length );
	str->cur_p=0;
	str->total_p=0;
	InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
	str = MQ_RefinementEncImage( RefImage, Image, RefDx, RefDy, codec, str, TpGDon, rTemplate, rATX1, rATY1, rATX2, rATY2);
	str = MQ_flush(codec, str);
	MQ_Eaddr = str->cur_p;

	for(i=0;i<MQ_Eaddr;i++)
		printf("%x,",str->buf[i]);

	str->cur_p=0;
	InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
	rImage = MQ_RefinementDecImage( RefImage, Image->width, Image->height, RefDx, RefDy, codec, str, MQ_Eaddr, TpGDon, rTemplate, rATX1, rATY1, rATX2, rATY2);
	rD_ = (uchar *)rImage->data;
	D_ = (uchar *)Image->data;
	Flag=0;
	for(j=0;j<Image->height;j++){
		for(i=0;i<Image->width;i++){
			if( rD_[j*col1step+i] != D_[j*col1step+i] ){
				printf("MQ_RefinementEncImage/MQ_RefinementDecImage ERROR!! %d %d %x %x\n",j,i,rD_[j*col1step+i], D_[j*col1step+i] );
				Flag=1;
			}
		}
	}
	if(!Flag)
		printf("MQ_RefinementEncImage/MQ_RefinementDecImage is OK\n");

	delete	[] rImage->Pdata;
	delete	rImage;

	printf("program end\n");
	return	TRUE;
//	exit(0);
}

