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
#include    <stdlib.h>
#include    <string.h>
#include	"ImageUtil.h"
#include	"Jb2Common.h"
#include	"T4T6codec.h"
#include	"Jb2_MQLapper.h"
#include	"Jb2_T4T6Lapper.h"
#include	"T45_codec.h"
#include	"Jb2_Debug.h"


void Jbig2_ImageMarg(struct Image_s *S, struct Image_s *T, uchar CombOP, byte4 Cur_T, byte4 Cur_S, uchar RefCorner, uchar ColourExtFlag, byte4 *Col )
{
	byte4	iS, iT, jT;
	byte4	height, width;
	uchar	*Dt_TS, *Dt_, *Ds_TS, *Ds_, ccc;

	height = T->tby1-T->tby0;
	width  = T->tbx1-T->tbx0;
	Ds_TS = (uchar *)S->data;
	Dt_TS = (uchar *)T->data;
	if(!ColourExtFlag){
		if(CombOP==JBIG2_OR){
			switch(RefCorner){
			case Jb2_BottomLeft://BottomLeft
				Ds_TS = &Ds_TS[(Cur_T-height+1)*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, ++Ds_, ++Dt_ )
						*Ds_ |= *Dt_;
				}
				break;
			case Jb2_TopLeft://TopLeft
				Ds_TS = &Ds_TS[Cur_T*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, ++Ds_, ++Dt_ )
						*Ds_ |= *Dt_;
				}
				break;
			case Jb2_BottomRight://BottomRight
				break;
			case Jb2_TopRight://TopRight
				break;
			default:
				break;
			}
		}
		else if(CombOP==JBIG2_XOR){
			switch(RefCorner){
			case Jb2_BottomLeft://BottomLeft
				Ds_TS = &Ds_TS[(Cur_T-height+1)*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, ++Ds_, ++Dt_ )
						*Ds_ ^= *Dt_;
				}
				break;
			case Jb2_TopLeft://TopLeft
				Ds_TS = &Ds_TS[Cur_T*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, ++Ds_, ++Dt_ )
						*Ds_ ^= *Dt_;
				}
				break;
			case Jb2_BottomRight://BottomRight
				break;
			case Jb2_TopRight://TopRight
				break;
			default:
				break;
			}
		}
	}
	else{
		if(S->row1step==3 && T->row1step==1){
			//CombOP do not care. Replace only. 
			switch(RefCorner){
			case Jb2_BottomLeft://BottomLeft
				Ds_TS = &Ds_TS[(Cur_T-height+1)*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S*S->row1step];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, Ds_+=S->row1step, Dt_+=T->row1step ){
						if(*Dt_){
							for(ccc=0 ; ccc<S->row1step ; ccc++){
								Ds_[ccc] = *Dt_*Col[ccc];
							}
						}
					}
				}
				break;
			case Jb2_TopLeft://TopLeft
				Ds_TS = &Ds_TS[Cur_T*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S*S->row1step];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, Ds_+=S->row1step, Dt_+=T->row1step ){
						if(*Dt_){
							for(ccc=0 ; ccc<S->row1step ; ccc++){
								Ds_[ccc] = *Dt_*Col[ccc];
							}
						}
					}
				}
				break;
			case Jb2_BottomRight://BottomRight
				break;
			case Jb2_TopRight://TopRight
				break;
			default:
				break;
			}
		}
		else if(S->row1step==3 && T->row1step==3){
			//CombOP do not care. Replace only. 
			switch(RefCorner){
			case Jb2_BottomLeft://BottomLeft
				Ds_TS = &Ds_TS[(Cur_T-height+1)*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S*S->row1step];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, Ds_+=S->row1step, Dt_+=T->row1step ){
						for(ccc=0 ; ccc<S->row1step ; ccc++){
							Ds_[ccc] = Dt_[ccc];
						}
					}
				}
				break;
			case Jb2_TopLeft://TopLeft
				Ds_TS = &Ds_TS[Cur_T*S->col1step];
				for( jT=0 ; jT<height ; jT++, Ds_TS += S->col1step, Dt_TS+=T->col1step ){
					Ds_ = &Ds_TS[Cur_S*S->row1step];
					Dt_ = Dt_TS;
					for(iS=Cur_S , iT=0 ; iT<width ; iT++, Ds_+=S->row1step, Dt_+=T->row1step ){
						for(ccc=0 ; ccc<S->row1step ; ccc++){
							Ds_[ccc] = Dt_[ccc];
						}
					}
				}
				break;
			case Jb2_BottomRight://BottomRight
				break;
			case Jb2_TopRight://TopRight
				break;
			default:
				break;
			}
		}
	}

}


void SymbolID_Create( struct StreamChain_s *str, byte4 *SymbolID_V, byte4 *SymbolID_L, byte4 *SymbolID_O, byte4 *RunCode_V, byte4 *RunCode_L, byte4 *RunCode_O, ubyte4 numSymbol)
{
	ubyte4	iii, kkk, length_Last, length_K;
	byte4	data;
	byte4	*Flag;
	ubyte4	numSymbol0;

	numSymbol0=numSymbol;
	Flag = new byte4 [35];
	memset(Flag, 0, sizeof(byte4)*35);

	iii=0;
	length_Last=0;
	data=0;
	do{
		kkk=-1;
		length_Last=0;
		data=0;
		do{
			kkk++;
			length_K = RunCode_L[ RunCode_O[kkk] ]-length_Last;
			if(length_K)
				data = (byte4 )( (data<<length_K) | Ref_nBits(str, length_K) );
			length_Last = RunCode_L[ RunCode_O[kkk] ];
		} while( data!=RunCode_V[ RunCode_O[kkk] ] );

		SymbolID_V[iii]=(data<<RunCode_O[kkk]) + Flag[ RunCode_O[kkk] ];
		SymbolID_L[iii]=RunCode_L[ RunCode_O[kkk] ]+RunCode_O[kkk];
		Flag[ RunCode_O[kkk] ]++;
		iii++;
		numSymbol0--;
	} while(numSymbol0);

	iii=0;
	kkk=0;
	length_K=1;
	do{
		for(kkk=0 ; kkk<numSymbol ; kkk++ ){
			if(length_K==(unsigned)SymbolID_L[kkk]){
				SymbolID_O[iii]=kkk;
				iii++;
			}
		}
		length_K++;
	} while(iii<numSymbol);
	delete	[] Flag;
}


void B3_RunCodeCreate( byte4 *RunCode_V, byte4 *RunCode_L, byte4 *RunCode_O, byte4 numCode )
{
	byte4	kkk;
	byte4	code=0, LengthT;

	kkk=0;
	while(RunCode_O[kkk]!=-1){
		LengthT=RunCode_L[ RunCode_O[kkk] ];
		while(LengthT==RunCode_L[ RunCode_O[kkk] ] ){
			RunCode_V[ RunCode_O[kkk] ]=code;
			code++;
			kkk++;
		}
		code *=2;
	}
}

//RunCode[0]:RunCode00, RunCode[1]:RunCode01, ..... ,RunCode[34]:RunCode34 
byte4 *B3_Sort( byte4 *RunCode_L, byte4 numCode )
{
	byte4	i, k;
	byte4	LengthT;
	byte4	*RunCode_O;

	RunCode_O = new byte4 [numCode];
	memset(RunCode_O, -1, sizeof(byte4)*numCode);

	LengthT=1;
	k=0;
	while(LengthT<16){
		for( i=0 ; i<numCode ; i++){
			if(RunCode_L[i]==LengthT){
				RunCode_O[k]=i;
				k++;
			}
		}
		LengthT++;
	}
	return	RunCode_O;
}

void	*JBIG2_DecMain(struct StreamChain_s *str)
{
	struct	ImageChain_s *ImagePage, *rImagePage=NULL;//rImagePage is type of "BIT1". type of "BIT1" is output format.	//ImagePage is type of "CHAR".
	struct	Jb2HuffmanTable_s *Huff;
	ubyte4	numPage;
	ubyte4	NumberOfSegments;
	byte4	Saddr;
	char	ErrorFlag=0;
	uchar	ColorExtFlag=0;

	//FileHeader
	for(str->cur_p=0 ; str->cur_p<8 ; str->cur_p++ ){
		if( str->buf[str->cur_p]!=JB2_FILE_HEADER_ID[str->cur_p] )
			ErrorFlag++;
	}
	if(ErrorFlag){
		printf("FileHeader ERROR!\n");
		exit(0);
	}
	str->cur_p++;
	numPage=Ref_4Byte(str);

	Saddr=str->cur_p;
	NumberOfSegments=SegmentNumCount(str);

	Huff = CreateHuffmanTable( DEC );
	str->cur_p=Saddr;
	ImagePage = SegmentDecode( str, Huff, NumberOfSegments, &ColorExtFlag );//ImagePage->Image

	ImagePage = ImageChainSearch( ImagePage, 0);
	do{
		rImagePage = ImageChainCreate(rImagePage);
		if(!ColorExtFlag)
			rImagePage->Image = (struct Image_s *)ImageCharToBit1(ImagePage->Image);
		else
			rImagePage->Image = ImagePage->Image;
		ImagePage = ImagePage->child;
	} while(ImagePage!=NULL);

	return	(void *)rImagePage;
}

byte4 SegmentNumCount(struct StreamChain_s *str)
{
	ubyte4	SegOrder;				//Fig25
	uchar	SegmentHeaderFlags;		//Fig25
	uchar	ReferedSegmentCount;	//Fig25
	ubyte4	ReferredSegOrder;		//Fig25
	ubyte4	SegPageAssociation;		//Fig25
	ubyte4	SegmentDataLength;		//Fig25
	uchar	PageSize4Byte, SegmentType;
	ubyte4	i;
	ubyte4	NumberOfSegments=0;
	ubyte4	ReferredSegmentNmuber;
	uchar	RefSegByte;

	while(str->cur_p<str->buf_length){
		//Segment number 7.2
		SegOrder=Ref_4Byte(str);

		//Segment header flags 7.2.3
		SegmentHeaderFlags=Ref_1Byte(str);
		PageSize4Byte = SegmentHeaderFlags & 0x40;
		SegmentType       = SegmentHeaderFlags&Segment_Type;
		if(SegOrder<=0x100)			RefSegByte=1;
		else if(SegOrder<=0x10000)	RefSegByte=2;
		else						RefSegByte=4;

		//Referred-to segment count and retention flags 7.2.4
		ReferedSegmentCount=Ref_1Byte(str);
		ReferredSegmentNmuber = ReferedSegmentCount>>5;
		if( ReferredSegmentNmuber==7 ){
			ReferredSegmentNmuber  = (Ref_1Byte(str)<<16);
			ReferredSegmentNmuber += Ref_2Byte(str);
			//Referred-to Segment Load 7.2.5
		}
		else{
			//Referred-to Segment Load 7.2.5
			if(RefSegByte==1){
				for(i=0;i<ReferredSegmentNmuber;i++)
					ReferredSegOrder=Ref_1Byte(str);
			}
			else if(RefSegByte==2){
				for(i=0;i<ReferredSegmentNmuber;i++)
					ReferredSegOrder=Ref_2Byte(str);
			}
			else{
				for(i=0;i<ReferredSegmentNmuber;i++)
					ReferredSegOrder=Ref_4Byte(str);
			}
		}

		//Segment Page Association 7.2.6
		if(PageSize4Byte)
			SegPageAssociation =Ref_4Byte(str);
		else
			SegPageAssociation =(ubyte4)Ref_1Byte(str);

		//Segment Data Length
		SegmentDataLength=Ref_4Byte(str);
		str->cur_p += SegmentDataLength;

		NumberOfSegments++;
	}
	return	NumberOfSegments;
}


struct ImageChain_s *SegmentDecode( struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, byte4 NumberOfSegments, uchar *ColorExtFlag )
{
	byte4	i, j;
	ubyte4	SegNo, PageAssociate;
	byte4	SegmentDataLength, SymCount;
	uchar	SegmentHeaderFlags;
	uchar	PageAssociationSize, SegmentType, RefSegCount, ReferredSegmentNmuber, RefSegByte, DeferNonRetain;
	char	PageFlag=1;
	ubyte4	*pReferSegNo;
	struct	mqcodec_s *codec;
	struct	SymbolDictionarySegment_s	*SymbolDic=NULL;
	struct	ColourPaletteSegment_s		*ColourPalette=NULL;
	struct	PatternDictionarySegment_s	*PatternDic=NULL;
	struct	PageInformationSegment_s	*PageInfo=NULL;
	struct	TextRegionSegment_s			*TextRegion=NULL;
	struct	HalftoneRegionSegment_s		*HalfRegion=NULL;
	struct	GenericRegionSegment_s		*GenericRegion=NULL;
	struct	ImageChain_s  *ImagePat=NULL, *ImageGen=NULL;
	struct	ImageChain_s *ImageTxt=NULL, *ImageHaf=NULL, *ImagePage=NULL;
	struct	Jb2_ImageChain_s *ImageSym=NULL;
#if JBIG2_DEBUG00
	void	*temptempD;
#endif
	codec = new struct mqcodec_s;
	codec->numCX = Number_CX;
	codec->index = new uchar [codec->numCX];
	InitMQ_Codec2( codec );

	SymCount=0;
	for(j=0 ; j<NumberOfSegments ; j++){
#if JBIG2_DEBUG00
		printf("[SegmentDecode] SegmentNumber=%d\t", j);
#endif
		//Segment number 7.2
		SegNo=Ref_4Byte(str);
		//Segment header flags 7.2.3
		SegmentHeaderFlags=Ref_1Byte(str);
		DeferNonRetain =(SegmentHeaderFlags&0x80)>>7;
		PageAssociationSize = (SegmentHeaderFlags & 0x40)>>6;
		SegmentType = (SegmentHeaderFlags & 0x3f) ;
		//Referred-to segment count and retention flags 7.2.4
		RefSegCount=Ref_1Byte(str);
		ReferredSegmentNmuber = RefSegCount>>5;
		if( ReferredSegmentNmuber==7 ){
			ReferredSegmentNmuber  = (Ref_1Byte(str)<<16);
			ReferredSegmentNmuber += Ref_2Byte(str);
			//Referred-to Segment Load 7.2.5
		}

		//Referred-to Segment Load
		if(ReferredSegmentNmuber){
			pReferSegNo = new ubyte4 [ReferredSegmentNmuber];
			if(SegNo<=0x100)		RefSegByte=1;
			else if(SegNo<=0x10000)	RefSegByte=2;
			else					RefSegByte=4;
			if(RefSegByte==1){
				for(i=0;i<(signed)ReferredSegmentNmuber;i++)
					pReferSegNo[i]=(ubyte4)Ref_1Byte(str);
			}
			else if(RefSegByte==2){
				for(i=0;i<(signed)ReferredSegmentNmuber;i++)
					pReferSegNo[i]=(ubyte4)Ref_2Byte(str);
			}
			else{
				for(i=0;i<(signed)ReferredSegmentNmuber;i++)
					pReferSegNo[i]=Ref_4Byte(str);
			}
		}

		//Segment Page Association 7.2.6
		if(PageAssociationSize)
			PageAssociate =Ref_4Byte(str);
		else
			PageAssociate =(ubyte4)Ref_1Byte(str);
		
		//Segment Data Length
		SegmentDataLength=Ref_4Byte(str);

		switch(SegmentType){
		case SYMBOL_DICTIONARY:
#if JBIG2_DEBUG00
			printf("Symbol Dic \n");
#endif
			if(SymbolDic==NULL){
				SymbolDic = new struct SymbolDictionarySegment_s;
				SymbolDic->numSymbol_File=0;
				SymbolDic->numSymbol_Page=0;
			}
			SymbolDic->PageAssociate=PageAssociate;
			if( (PageAssociate==0) && (SymCount!=0) ){
				ImageSym=Jb2_ImageChainParentSearch(ImageSym);
				SymCount=0;
				while(ImageSym->child!=NULL){
					ImageSym->ID=-1;
					ImageSym->FileID=0;
					ImageSym = ImageSym->child;
				};
				ImageSym->ID=-1;
				ImageSym->FileID=0;
				SymbolDic->numSymbol_File=0;
			}
			SymbolDic->numReferSeg=ReferredSegmentNmuber;
			SymbolDic->SegNo=SegNo;
			SymbolDic->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	SymbolDic->pReferSegNo=pReferSegNo;
			else						SymbolDic->pReferSegNo=NULL;
			ImageSym = SymbolDictionarySegmentDec( &SymCount, ImageSym, ImageTxt, SymbolDic, str, Huff, codec, PageFlag );
			break;
		case INTERMEDIATE_TEXT_REGION:
		case IMMEDIATE_TEXT_REGION:
		case IMMEDIATE_LOSLESS_TEXT_REGION:
#if JBIG2_DEBUG00
			printf( "[SegmentDecode] Text Region \n ");
#endif
			if(TextRegion==NULL)
				TextRegion = new struct TextRegionSegment_s;
			TextRegion->PageAssociate=PageAssociate;
			TextRegion->numReferSeg=ReferredSegmentNmuber;
			TextRegion->SegNo=SegNo;
			TextRegion->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	TextRegion->pReferSegNo=pReferSegNo;
			else						TextRegion->pReferSegNo=NULL;
			/*ImageTxt =*/ TextRegionSegmentDec( ImagePage, /*ImageTxt,*/ ImageSym, TextRegion, str, Huff, codec, (SymbolDic->numSymbol_File+SymbolDic->numSymbol_Page) );
			break;
		case PATTERN_DICTIONARY:
#if JBIG2_DEBUG00
			printf("[SegmentDecode] Pattern Dic \n");
#endif
			if(PatternDic==NULL){
				PatternDic = new struct	PatternDictionarySegment_s;
				PatternDic->numPattern=0;
			}
			PatternDic->PageAssociate=PageAssociate;
			PatternDic->numReferSeg=ReferredSegmentNmuber;
			PatternDic->SegNo=SegNo;
			PatternDic->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	PatternDic->pReferSegNo=pReferSegNo;
			else						PatternDic->pReferSegNo=NULL;
#if JBIG2_DEBUG00
			temptempD = (void *)PatternDic;
			printf("[SegmentDecode] ImagePat = 0x%x PatternDic = 0x%x \n", ImagePat, temptempD);
#endif
			ImagePat = PatternDictionarySegmentDec( ImagePat, str, PatternDic, Huff, codec );
			break;
		case INTERMIDIATE_HALFTONE_REGION:
		case IMMEDIATE_HALFTONE_REGION:
		case IMMEDIATE_LOSLESS_HALFTONE_REGION:
#if JBIG2_DEBUG00
			printf( "[SegmentDecode] Halftone Region \n");
#endif
			if(HalfRegion==NULL)
				HalfRegion = new struct HalftoneRegionSegment_s;
			HalfRegion->PageAssociate=PageAssociate;
			HalfRegion->numReferSeg=ReferredSegmentNmuber;
			HalfRegion->SegNo=SegNo;
			HalfRegion->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	HalfRegion->pReferSegNo=pReferSegNo;
			else						HalfRegion->pReferSegNo=NULL;
#if JBIG2_DEBUG00
			temptempD = (void *)HalfRegion;
			printf("[SegmentDecode] HalfRegion = 0x%x \n", temptempD);
#endif
			ImageHaf = HalftoneRegionSegmentDec( ImagePage, ImageHaf, ImagePat, str, HalfRegion, Huff, codec, PatternDic->numPattern, PageInfo->ColorExtFlag );
#if JBIG2_DEBUG00
			printf(" [SegmentDecode] Halftone Region complete\n");
#endif
			break;
		case INTERMEDIATE_GENERIC_REFINMENT_REGION:
		case IMMEDIATE_GENERIC_REFINMENT_REGION:
		case IMMEDIATE_LOSLESS_GENERIC_REFINMENT_REGION:
		case INTERMEDIATE_GENERIC_REGION:
		case IMMEDIATE_GENERIC_REGION:
		case IMMEDIATE_LOSLESS_GENERIC_REGION:
#if JBIG2_DEBUG00
			printf( "Generic Region \n");
#endif
			if(GenericRegion==NULL)
				GenericRegion = new struct GenericRegionSegment_s;
			GenericRegion->PageAssociate=PageAssociate;
			GenericRegion->numReferSeg=ReferredSegmentNmuber;
			GenericRegion->SegNo=SegNo;
			GenericRegion->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	GenericRegion->pReferSegNo=pReferSegNo;
			else						GenericRegion->pReferSegNo=NULL;
			ImageGen = GenericRegionSegmentDec( ImagePage, ImageGen, str, GenericRegion, Huff, codec, PageInfo->ColorExtFlag );
			break;
		case PAGE_INFORMATION:
#if JBIG2_DEBUG00
			printf( "PageInformation \n");
#endif
			if(PageInfo==NULL)
				PageInfo = new struct PageInformationSegment_s;
			PageInfo->PageAssociate=PageAssociate;
			PageInfo->numReferSeg=ReferredSegmentNmuber;
			PageInfo->SegNo=SegNo;
			PageInfo->SegmentDataLength = SegmentDataLength;
			if(ReferredSegmentNmuber)	PageInfo->pReferSegNo=pReferSegNo;
			else						PageInfo->pReferSegNo=NULL;
			ImagePage = PageInformationSegmentDec( ImagePage, PageInfo, str, ColorExtFlag );
			*ColorExtFlag = PageInfo->ColorExtFlag;
			PageFlag=0;
			break;
		case COLOUR_PALETTE:
#if JBIG2_DEBUG00
			printf("Color Palette \n");
#endif
			ColourPaletteSegmentDec( ColourPalette, str );
			break;
		case END_OF_PAGE:
#if JBIG2_DEBUG00
			printf("EndOfPage \n");
#endif
			EndOfPageSegmentDec( ImagePage, (PageAssociate-1) );
			if(SymbolDic!=NULL)
				SymbolDic->numSymbol_Page=0;
			if(ImageSym!=NULL){
				ImageSym=Jb2_ImageChainParentSearch(ImageSym);
				SymCount=0;
				while(ImageSym->child!=NULL){
					if( ImageSym->FileID ){
						SymCount = ImageSym->ID;
						SymCount++;
					}
					else	ImageSym->ID=-1;
					ImageSym = ImageSym->child;
				};
				if( ImageSym->FileID ){
					SymCount= ImageSym->ID;
					SymCount++;
				}
				else	ImageSym->ID=-1;
			}			
			PageFlag=1;
			break;
		case END_OF_STRIPE:
			break;
		case END_OF_FILE:
			break;
		case PROFILES:
			break;
		case TABLES:
			break;
		case EXTENSION:
			break;
		default:
			break;
		}
	}
	return	ImagePage;
}

//7.4.2
struct Jb2_ImageChain_s *SymbolDictionarySegmentDec( byte4 *SymCount, struct Jb2_ImageChain_s *ImageSym, struct ImageChain_s *ImageTxt, struct SymbolDictionarySegment_s *SymbolDic, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, char PageFlag )
{
	byte4	DeltaWidth, TotalWidth, xbyte, Width[256], sWidth, eWidth, numWidth;
	byte4	Height, DeltaHeight;
	byte4	BmSize, SdNumExSyms, SdNumNewSyms, RefAggnInst;
	byte4	i, j, k, iii, numSymbol, numSymbol1, SymbolCodeLength, numAggregate;
	byte4	RefID, RefDx=0, RefDy=0, StripT0=1;
	byte4	MQ_Eaddr;
	byte4	tempD1, tempD2;
	ubyte2	Flags;
	char	ATX1=0, ATY1=0, ATX2=0, ATY2=0, ATX3=0, ATY3=0, ATX4=0, ATY4=0, rATX1=0, rATY1=0, rATX2=0, rATY2=0;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0, ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;
	char	Template, rTemplate, ExtTemplate=0, RefinementAggregate, Huff_Selection, BitmapCodingContextUsed, BitmapCodingContextRetained;
	uchar	DsOffset=0, /*SbNumInstances,*/ numCode=35;
	uchar	DH_Huff, DW_Huff, Bm_Huff, Ag_Huff, sHuff;
	uchar	Flag=0, TpGDon=0;
	uchar	*D_TS, *D_, *tempD, tempDD;
	struct	Image_s *RefImage, *AggImage;


	MQ_Eaddr = str->cur_p+SymbolDic->SegmentDataLength;

	Flags = Ref_2Byte(str);
	sHuff = (Flags & 1);
	RefinementAggregate = (Flags & 2)>>1;
	Huff_Selection = (Flags & 0xc)>>2;
	switch( Huff_Selection ){
	case 0:DH_Huff=3;	break;//TableB4
	case 1:DH_Huff=4;	break;//TableB5
	case 3:DH_Huff=15;	break;
	default:			break;
	}

	Huff_Selection = (Flags & 0x30)>>4;
	switch( Huff_Selection ){
	case 0:	DW_Huff=1;	break;//TableB2
	case 1:	DW_Huff=2;	break;//TableB3
	case 3:	DW_Huff=15;	break;
	default:			break;
	}

	Huff_Selection = (Flags & 0x40)>>6;
	if( !Huff_Selection )	Bm_Huff=0;//TableB1
	else					Bm_Huff=15;

	Huff_Selection = (Flags & 0x80)>>7;
	if( !Huff_Selection )	Ag_Huff=0;//TableB1
	else					Ag_Huff=15;

	BitmapCodingContextUsed = (Flags & 0x100)>>8;
	BitmapCodingContextRetained = (Flags & 0x200)>>9;
	Template = (Flags & 0xc00)>>10;
	rTemplate = (Flags & 0x100)>>12;
	if(!sHuff){
		//Symbol Dictionary AT flags
		if(!Template){
			ATX1 = (char)Ref_1Byte(str);
			ATY1 = (char)Ref_1Byte(str);
			ATX2 = (char)Ref_1Byte(str);
			ATY2 = (char)Ref_1Byte(str);
			ATX3 = (char)Ref_1Byte(str);
			ATY3 = (char)Ref_1Byte(str);
			ATX4 = (char)Ref_1Byte(str);
			ATY4 = (char)Ref_1Byte(str);
		}
		else{
			ATX1 = (char)Ref_1Byte(str);
			ATY1 = (char)Ref_1Byte(str);
		}
	}

	if( RefinementAggregate && (!rTemplate) ){
		//Symbol Dictionary Refinement AT flags
		rATX1= (char)Ref_1Byte(str);
		rATY1= (char)Ref_1Byte(str);
		rATX2= (char)Ref_1Byte(str);
		rATY2= (char)Ref_1Byte(str);
	}

	//Number of Exported Symbol
	SdNumExSyms  = Ref_4Byte(str);

	SdNumNewSyms = Ref_4Byte(str);
	numSymbol1 = numSymbol = SdNumNewSyms;
	if(!PageFlag)
		SymbolDic->numSymbol_Page += SdNumNewSyms;
	else
		SymbolDic->numSymbol_File += SdNumNewSyms;
	memset(Width, 0, sizeof(byte4)*256);

	for( SymbolCodeLength=31 ; SymbolCodeLength>0 ; SymbolCodeLength--){
		if(	mask5[SymbolCodeLength]&SdNumExSyms ){
			if( (mask6[SymbolCodeLength] & SdNumExSyms) ){
				SymbolCodeLength++;
				break;
			}
			else
				break;
		}
	}

	if(RefinementAggregate){
		if(sHuff){
		}
		else{
			Flag=1;
			InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
			Height=0;
			i=0;
			while(numSymbol){
				//refine HeightClass loop
				DeltaHeight = MQ_DecInteger( str, codec, IADH, MQ_Eaddr, &Flag );	//IADH
				Height += DeltaHeight;
				Width[i] = MQ_DecInteger( str, codec, IADW, MQ_Eaddr, &Flag );					//IADW
				RefAggnInst = MQ_DecInteger( str, codec, IAAI, MQ_Eaddr, &Flag );				//IAAI
				while(RefAggnInst){//RefAggnInst is number of Symbol made by refinement in this HeightClass.
					RefID = MQ_DecIntegerIAID( str, codec, MQ_Eaddr, SymbolCodeLength, IAID );	//IAID
					ImageSym = Jb2_ImageChainSearch( ImageSym, RefID );
					RefImage = ImageSym->Image;
					RefDx = MQ_DecInteger( str, codec, IARDX, MQ_Eaddr, &Flag );				//IARDX
					RefDy = MQ_DecInteger( str, codec, IARDY, MQ_Eaddr, &Flag );				//IARDY
					ImageSym = Jb2_ImageChainCreate( ImageSym );
					ImageSym->Image = MQ_RefinementDecImage( RefImage, Width[i], Height, RefDx, RefDy, codec, str, MQ_Eaddr, TpGDon, rTemplate, rATX1, rATY1, rATX2, rATY2);
					ImageSym->ID= *SymCount;
					(*SymCount)++;
					if(PageFlag)	ImageSym->FileID = 1;
#if JBIG2_DEBUG01
					Jb2_Sym_Debug_Print( ImageSym, ImageSym->ID, SymbolDic->SegNo );
#endif
					RefAggnInst--;
					numSymbol--;
				}
				if(numSymbol){
					DeltaWidth = MQ_DecInteger( str, codec, IADW, MQ_Eaddr, &Flag );
					Width[i+1] = DeltaWidth + Width[i];
					i++;
					numAggregate = MQ_DecInteger( str, codec, IAAI, MQ_Eaddr, &Flag );
					AggImage = ImageCreate( NULL, Width[i], Height, 0, Width[i], 0, Height, CHAR);
					SymbolAggregate_MQDec( AggImage, ImageSym, str, RefinementAggregate, StripT0, Jb2_TopLeft, JBIG2_OR, DsOffset, numAggregate, numCode, codec, MQ_Eaddr, SdNumExSyms, 0, rATX1, rATY1, rATX2, rATY2, 0, NULL );
					ImageSym = Jb2_ImageChainCreate( ImageSym );
					ImageSym->Image = AggImage;
					ImageSym->ID= *SymCount;
					(*SymCount)++;
					if(PageFlag)	ImageSym->FileID = 1;
#if JBIG2_DEBUG01
					Jb2_Sym_Debug_Print( ImageSym, ImageSym->ID, SymbolDic->SegNo );
#endif
					numSymbol--;
				}
			}
			tempD1 = MQ_DecInteger( str, codec, IAEX, MQ_Eaddr, &Flag );
			tempD2 = MQ_DecInteger( str, codec, IAEX, MQ_Eaddr, &Flag );
		}
		str = ByteStuffOutJXR(str);
	}
	else{
		//No (Refinement & Aggregate) and Huff
		if(sHuff){
			Height =0;
			while(numSymbol){ //HeightClass loop
				memset(Width, 0, sizeof(byte4)*256);
				DeltaHeight = JBIG2_HuffDec(str, &Huff[DH_Huff] );
				Height += DeltaHeight;
				i=0;
				while(1){
					DeltaWidth = JBIG2_HuffDec(str, &Huff[DW_Huff] );
					if(DeltaWidth!=OOB){
						Width[i] += DeltaWidth;
						i++;
						Width[i] = Width[i-1];
						numSymbol--;
					}
					else{
						Width[i]=OOB;	numWidth=i;	break;
					}
				}

				//SDHUFFBMSIZE
				BmSize = JBIG2_HuffDec(str, &Huff[Bm_Huff] );
				Ref_StuffBits(str, NoDiscard );

				if(BmSize){
					for( k=0 ; k<numWidth ; k++ ){
						ImageSym = Jb2_ImageChainCreate(ImageSym);
						ImageSym->Image = T4T6DecMain( NULL, str, T6, MSBFirst, Width[k], Height);
						ImageSym->ID= *SymCount;
						(*SymCount)++;
						if(PageFlag)	ImageSym->FileID =1;
						ImageSym->Image->MaxValue=-1;
#if JBIG2_DEBUG01
						Jb2_Sym_Debug_Print( ImageSym, ImageSym->ID, SymbolDic->SegNo );
#endif
					}
				}
				else{
					TotalWidth=0;
					for( k=0 ; k<numWidth ; k++)
						TotalWidth+=Width[k];
					xbyte = ceil2( TotalWidth, 8 );
					tempD = new  uchar [TotalWidth*Height]; 
					for( j=0 ; j<Height ; j++ ){
						for( i=0 ; i<TotalWidth ; i++ ){
							iii=i%8;
							if(iii==0)
								tempDD=Ref_1Byte(str);
							tempD[j*TotalWidth+i] = (tempDD>>(7-iii))&1;
						}
					}
					for( k=0 ; k<numWidth ; k++ ){//
						ImageSym = Jb2_ImageChainCreate(ImageSym);
						ImageSym->Image = ImageCreate(NULL, Width[k], Height, 0, Width[k], 0, Height, CHAR);
						ImageSym->ID= *SymCount;
						(*SymCount)++;
						if(PageFlag)	ImageSym->FileID = 1;
						ImageSym->Image->MaxValue=-1;
						D_TS = (uchar *)ImageSym->Image->data;
						for( j=0 ; j<Height ; j++, D_TS+=ImageSym->Image->col1step ){
							D_= D_TS;
							sWidth =(byte4)Dout4(Width, (k-1), (signed)SdNumNewSyms );
							eWidth =sWidth+Width[k]; 
							for( i=0, iii=sWidth ; i<Width[k] ; i++, iii++)
								D_[i]=tempD[j*TotalWidth+iii];
						}
#if JBIG2_DEBUG01
						Jb2_Sym_Debug_Print( ImageSym, ImageSym->ID, SymbolDic->SegNo);
#endif
					}
				}
			}
			Ref_1Byte(str);
			Ref_1Byte(str);
		}
		//No (Refinement & Aggregate) and Arithemetric
		else{
			Height = 0;
			InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
			while(numSymbol){ //loop by HeightClass
				DeltaHeight = MQ_DecInteger( str, codec, IADH, MQ_Eaddr, &Flag );	//IADH
				Height += DeltaHeight;
				i=0;
				Flag=1;
				memset(Width, 0, sizeof(byte4)*256);
				while(1){
					DeltaWidth = MQ_DecInteger( str, codec, IADW, MQ_Eaddr, &Flag );	//IADW
					if(!Flag)	break;
					Width[i] +=DeltaWidth;
					ImageSym = Jb2_ImageChainCreate(ImageSym);
					ImageSym->Image = MQ_DecImage( Width[i], Height, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
					ImageSym->ID= *SymCount;
					(*SymCount)++;
					if(PageFlag)
						ImageSym->FileID = 1;
					ImageSym->Image->MaxValue=-1;	//White:0 Black:1 in JBIG2
#if JBIG2_DEBUG01
					Jb2_Sym_Debug_Print( ImageSym, ImageSym->ID, SymbolDic->SegNo );
#endif
					numSymbol--;
					i++;
					Width[i] = Width[i-1];
				}
			}
			tempD1 = MQ_DecInteger( str, codec, IAEX, MQ_Eaddr, &Flag );	//IAEX
			tempD2 = MQ_DecInteger( str, codec, IAEX, MQ_Eaddr, &Flag );	//IAEX
			str = ByteStuffOutJXR(str);
		}
	}
	return	ImageSym;
}

void TextRegionDec_Huff_Kernel( struct Image_s *Image, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, uchar sHuff, uchar LogSbStrips, uchar RefCorner, uchar CombOp, uchar DsOffset, byte4 SbNumInstances, byte4 numCode, uchar Dt_Huff, uchar Fs_Huff, uchar Ds_Huff, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 MQ_Eaddr, ubyte4 numSymbol, uchar ColourExtFlag, byte4 *Col)
{
	byte4	i, nInstances, StripT, StripT0;
	byte4	*RunCode_L=NULL, *RunCode_V=NULL, *SymbolID_V=NULL, *SymbolID_L=NULL, *SymbolID_O=NULL, *RunCode_O=NULL;
	byte4	deltaT, deltaS;
	byte4	Cur_S, Cur_T;
	byte4	ID, textCount=0;
	uchar	Flag=1, FirstS=1;
	ubyte2	tempD;

	//SymbolID
	RunCode_L = new byte4 [numCode];
	RunCode_V = new byte4 [numCode];
	memset(RunCode_L, 0, sizeof(byte4)*numCode);
	memset(RunCode_V, 0, sizeof(byte4)*numCode);
	SymbolID_V = new byte4 [SbNumInstances];
	SymbolID_L = new byte4 [SbNumInstances];
	SymbolID_O = new byte4 [SbNumInstances];
	memset( SymbolID_V, 0, sizeof(byte4)*SbNumInstances );
	memset( SymbolID_L, 0, sizeof(byte4)*SbNumInstances );
	memset( SymbolID_O, -1, sizeof(byte4)*SbNumInstances );
	for(i=0;i<17;i++){
		tempD = Ref_1Byte(str);
		RunCode_L[i*2+0]=(tempD>>4)&0xf;
		RunCode_L[i*2+1]=(tempD)&0xf;
	}
	RunCode_L[34] = Ref_nBits(str, 4);
	RunCode_O = B3_Sort( RunCode_L, numCode);
	B3_RunCodeCreate( RunCode_V, RunCode_L, RunCode_O, numCode );
	SymbolID_Create( str, SymbolID_V, SymbolID_L, SymbolID_O, RunCode_V, RunCode_L, RunCode_O, numSymbol);
	str = ByteStuffOutJXR(str);

	StripT0=(1<<LogSbStrips);
	StripT = StripT0;
	Cur_T=0;
	//Initial StripT value
	deltaT = JBIG2_HuffDec(str, &Huff[Dt_Huff] );
	StripT = StripT0 * deltaT * (-1);			//(j-1)"0" STRIPT=-4

	nInstances=0;
	do{
		//DeltaT
		deltaT = JBIG2_HuffDec(str, &Huff[Dt_Huff] );
		StripT = StripT + deltaT * StripT0;
		while( 1 ){
			//Instance S
			if(FirstS){
				//First never occor OOB
				deltaS= JBIG2_HuffDec(str, &Huff[Fs_Huff] );
				Cur_S = deltaS;
				FirstS=0;
			}
			else{
				//Second- 
				deltaS = JBIG2_HuffDec(str, &Huff[Ds_Huff] );
				if(deltaS==OOB)
					break;
				Cur_S = Cur_S + deltaS + DsOffset;
			}
			//Instance T
			deltaT = Ref_nBits(str, LogSbStrips);
			Cur_T = deltaT + StripT0;

			//SymbolID
			ID = JBIG2_ID_Dec( SymbolID_V, SymbolID_L, SymbolID_O, str );
			ImageSym = Jb2_ImageChainSearch( ImageSym, ID);
			Jbig2_ImageMarg( Image, ImageSym->Image, CombOp, Cur_T, Cur_S, RefCorner, ColourExtFlag, &Col[textCount]);
			textCount++;
			Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
			nInstances++;
		} ;
	} while(nInstances<SbNumInstances);
}

void SymbolAggregate_MQDec( struct Image_s *AggImage, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, uchar RefinementAggregate, uchar StripT0, uchar RefCorner, uchar CombOp, uchar DsOffset, byte4 SbNumInstances, byte4 numCode, struct mqcodec_s *codec, byte4 MQ_Eaddr, ubyte4 numSymbol, uchar Template, char RATX1, char RATY1, char RATX2, char RATY2, uchar ColourExtFlag, byte4 *Col)
{
	struct	Image_s *RefImage;
	byte4	nInstances, StripT;
	byte4	deltaT, deltaS;
	byte4	Cur_S, Cur_T;
	byte4	SymbolCodeLength;
	byte4	RefID, ID, Ri, RDw, RDh, RDx, RDy, RefDx, RefDy;
	byte4	textCount=0;
	uchar	Flag=1, FirstS=1, TpGDon=0;
#if JBIG2_DEBUG05
	struct Image_s *rImage;
#endif

	StripT = StripT0;
	Cur_T=0;

	for( SymbolCodeLength=31 ; SymbolCodeLength>0 ; SymbolCodeLength--){
		if(	mask5[SymbolCodeLength]&numSymbol ){
			if( (mask6[SymbolCodeLength] & numSymbol) ){
				SymbolCodeLength++;
				break;
			}
			else
				break;
		}
	}

	//Initial StripT value
	deltaT = MQ_DecInteger( str, codec, IADT, MQ_Eaddr, &Flag );
	StripT = StripT0 * deltaT * (-1);
	nInstances=0;
	do{
		//DeltaT
		deltaT = MQ_DecInteger( str, codec, IADT, MQ_Eaddr, &Flag );
		StripT += (deltaT * StripT0);
		Flag=1;
		while( 1 ){
			//Instance S
			if(FirstS){
				deltaS = MQ_DecInteger( str, codec, IAFS, MQ_Eaddr, &Flag );
				Cur_S = deltaS;
				FirstS=0;
			}
			else{
				deltaS = MQ_DecInteger( str, codec, IADS, MQ_Eaddr, &Flag );
				if(!Flag)
					break;
				Cur_S = Cur_S + deltaS + DsOffset;
			}

			//Instance T
			if((!RefinementAggregate) && (StripT0!=1)){
				deltaT = MQ_DecInteger( str, codec, IAIT, MQ_Eaddr, &Flag );
				Cur_T = deltaT + StripT0;
			}

			//SymbolID
			ID = MQ_DecIntegerIAID(str, codec, MQ_Eaddr, SymbolCodeLength, IAID );
			if(RefinementAggregate){
				RefID = ID;
				Ri = MQ_DecInteger( str, codec, IARI, MQ_Eaddr, &Flag );
				if(Ri){
					ImageSym = Jb2_ImageChainSearch( ImageSym, RefID );
					RDw = MQ_DecInteger( str, codec, IARDW, MQ_Eaddr, &Flag );
					RDh = MQ_DecInteger( str, codec, IARDH, MQ_Eaddr, &Flag );
					RDx = MQ_DecInteger( str, codec, IARDX, MQ_Eaddr, &Flag );
					RDy = MQ_DecInteger( str, codec, IARDY, MQ_Eaddr, &Flag );
					RefDx = floor2(RDw, 2) + RDx;
					RefDy = floor2(RDh, 2) + RDy;
					RDw += (ImageSym->Image->tbx1-ImageSym->Image->tbx0);
					RDh += (ImageSym->Image->tby1-ImageSym->Image->tby0);
					RefImage = MQ_RefinementDecImage( ImageSym->Image, RDw, RDh, RefDx, RefDy, codec, str, MQ_Eaddr, TpGDon, Template, RATX1, RATY1, RATX2, RATY2 );
					Jbig2_ImageMarg( AggImage, RefImage, CombOp, Cur_T, Cur_S, RefCorner, ColourExtFlag, &Col[textCount*3]);
#if JBIG2_DEBUG05
					char	fname[256];
					strcpy(fname, "AggImage000");
					fname[10]=0x30+(textCount%10);
					fname[ 9]=0x30+((textCount/10) %10);
					fname[ 8]=0x30+((textCount/100) %10);
					strcat(fname, ".bmp");
					rImage = (struct Image_s *)ImageCharToBit1(AggImage);
					SaveBmp777(fname, rImage);
#endif
					textCount++;
					Cur_S += (RefImage->tbx1-RefImage->tbx0-1);
				}
				else{
					ImageSym = Jb2_ImageChainSearch( ImageSym, ID);
					Jbig2_ImageMarg( AggImage, ImageSym->Image, CombOp, Cur_T, Cur_S, RefCorner, ColourExtFlag, &Col[textCount*3]);
#if JBIG2_DEBUG05
					char	fname[256];
					strcpy(fname, "AggImage000");
					fname[10]=0x30+(textCount%10);
					fname[ 9]=0x30+((textCount/10) %10);
					fname[ 8]=0x30+((textCount/100) %10);
					strcat(fname, ".bmp");
					rImage = (struct Image_s *)ImageCharToBit1(AggImage);
					SaveBmp777(fname, rImage);
#endif
					textCount++;
					Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
				}
			}
			else{
				ImageSym = Jb2_ImageChainSearch( ImageSym, ID);
				Jbig2_ImageMarg( AggImage, ImageSym->Image, CombOp, Cur_T, Cur_S, RefCorner, ColourExtFlag, &Col[textCount*3]);
#if JBIG2_DEBUG05
				char	fname[256];
				strcpy(fname, "AggImage000");
				fname[10]=0x30+(textCount%10);
				fname[ 9]=0x30+((textCount/10) %10);
				fname[ 8]=0x30+((textCount/100) %10);
				strcat(fname, ".bmp");
				rImage = (struct Image_s *)ImageCharToBit1(AggImage);
				SaveBmp777(fname, rImage);
#endif
				textCount++;
				Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
			}
			nInstances++;
		};
	} while(nInstances<SbNumInstances);

#if JBIG2_DEBUG05
	char	fname[256];
	strcpy(fname, "AggImage");
	strcat(fname, ".bmp");
	rImage = (struct Image_s *)ImageCharToBit1(AggImage);
	SaveBmp777(fname, rImage);
#endif
}

//7.4.3
byte4	TextRegionSegmentDec( struct ImageChain_s *ImagePage, /*struct ImageChain_s *ImageTxt,*/ struct Jb2_ImageChain_s *ImageSym, struct TextRegionSegment_s *TextRegion, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, ubyte4 numSymbol/*, uchar temptemp*/ )
{
	byte4	numCode=35;
	byte4	BitmapWidth, BitmapHeight;
	byte4	*RunCode_L=NULL, *RunCode_V=NULL, *SymbolID_V=NULL, *SymbolID_L=NULL, *SymbolID_O=NULL, *RunCode_O=NULL;
	byte4	SbNumInstances;
	byte4	MQ_Eaddr, ColorLength_Addr, ColorLength, tempAddr;
	byte4	*Col=NULL, *ONE=NULL;
	uchar	sHuff, RefCorner, CombOp, LogSbStrips, DsOffset, StripT0;
	uchar	Huff_Selection, HuffRdWSelection, HuffRdHSelection, HuffRdXSelection, HuffRdYSelection, HuffRSizeSelection;
	uchar	Fs_Huff, Ds_Huff, Dt_Huff;
	uchar	Flag;
	char	RATX1=0, RATY1=0, RATX2=0, RATY2=0;
	struct	Image_s *ImageTxt=NULL;
	ubyte2	tempD;

	MQ_Eaddr = str->cur_p+TextRegion->SegmentDataLength;
	tempAddr = str->cur_p;

	ColorLength_Addr = MQ_Eaddr-4;
	str->cur_p = ColorLength_Addr;
	ColorLength = Ref_4Byte( str );
	str->cur_p = tempAddr;

	//RegionSegment
	BitmapWidth  = Ref_4Byte(str);
	BitmapHeight = Ref_4Byte(str);
	TextRegion->RegionSegmentXlocation    = Ref_4Byte(str);
	TextRegion->RegionSegmentYlocation    = Ref_4Byte(str);
	Flag = Ref_1Byte(str);
	TextRegion->ExternalCombinationOperator=Flag&7;
	TextRegion->ColourExtFlag = (Flag&8)>>3;
	if(TextRegion->ColourExtFlag){
		ONE = new byte4 [3];
		ONE[0]=ONE[1]=ONE[2]=1;
		MQ_Eaddr = MQ_Eaddr - ColorLength;
		tempAddr = str->cur_p;
		str->cur_p = MQ_Eaddr;
		Col = T45_Dec(str);
		str->cur_p = tempAddr;
	}

	//TextRegionSegmentFlag
	tempD = Ref_2Byte(str);
	sHuff = tempD&1;
	TextRegion->Refine=(tempD&2)>>1;
	LogSbStrips=(tempD&0xc)>>2;
	StripT0 = (1<<LogSbStrips);
	RefCorner  =(tempD&0x30)>>4;
	TextRegion->Transposed =(tempD&0x40)>>6;
	CombOp   =(tempD&0x180)>>7;
	TextRegion->SbDefPixel =(tempD&0x200)>>9;
	DsOffset =(tempD&0x7c00)>>10;
	TextRegion->SbrTemplate =(tempD&0x8000)>>15;

	if(sHuff){
		tempD = Ref_2Byte(str);
		Huff_Selection =(tempD&3);
		switch(Huff_Selection){
		case 0:	Fs_Huff= 5;	break;//TableB6
		case 1:	Fs_Huff= 6;	break;//TableB7
		case 3:	Fs_Huff=15;	break;
		default:			break;
		}

		Huff_Selection =(tempD&0xc)>>2;
		switch(Huff_Selection){
		case 0:	Ds_Huff= 7;	break;//TableB8
		case 1:	Ds_Huff= 8;	break;//TableB9
		case 2: Ds_Huff= 9;	break;//TableB10
		case 3:	Ds_Huff=15;	break;
		default:			break;
		}

		Huff_Selection =(tempD&0x30)>>4;
		switch(Huff_Selection){// Huff_Dt Arith_IADT
		case 0:	Dt_Huff=10;	break;//TableB11
		case 1:	Dt_Huff=11;	break;//TableB12
		case 2: Dt_Huff=12;	break;//TableB13
		case 3:	Dt_Huff=15;	break;
		default:			break;
		}
		HuffRdWSelection=(tempD&0xc0)>>6;
		HuffRdHSelection=(tempD&0x300)>>8;
		HuffRdXSelection=(tempD&0xc00)>>10;
		HuffRdYSelection=(tempD&0x3000)>>12;
		HuffRSizeSelection = (tempD&0x4000)>>14;
	}

	if(TextRegion->Refine && (!TextRegion->SbrTemplate) ){
		RATX1=Ref_1Byte(str);
		RATY1=Ref_1Byte(str);
		RATX2=Ref_1Byte(str);
		RATY2=Ref_1Byte(str);
	}

	SbNumInstances = Ref_4Byte(str);

//	ImageTxt = ImageChainCreate(ImageTxt);
	if(!TextRegion->ColourExtFlag){
		ImageTxt = ImageCreate( ImageTxt, BitmapWidth, BitmapHeight, 0, BitmapWidth, 0, BitmapHeight, CHAR);
		memset(ImageTxt->data, 0, sizeof(uchar)*(ImageTxt->height*ImageTxt->col1step) );
	}
	else{
		ImageTxt = ImageCreate( NULL, BitmapWidth*3, BitmapHeight, 0, BitmapWidth, 0, BitmapHeight, CHAR);
		memset(ImageTxt->data, 0xff, sizeof(uchar)*(ImageTxt->height*ImageTxt->col1step) );
		ImageTxt->row1step=3;
		ImageTxt->MaxValue=255;
	}
	if(sHuff)
		TextRegionDec_Huff_Kernel( ImageTxt, ImageSym, str, sHuff, LogSbStrips, RefCorner, CombOp, DsOffset, SbNumInstances, numCode, Dt_Huff, Fs_Huff, Ds_Huff, Huff, codec, MQ_Eaddr, numSymbol, TextRegion->ColourExtFlag, Col);
	else{
		InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
		SymbolAggregate_MQDec( ImageTxt, ImageSym, str, TextRegion->Refine, StripT0, RefCorner, CombOp, DsOffset, SbNumInstances, numCode, codec, MQ_Eaddr, numSymbol, TextRegion->SbrTemplate, RATX1, RATY1, RATX2, RATY2, TextRegion->ColourExtFlag, Col);
	}
	str = ByteStuffOutJXR(str);
	if(TextRegion->ColourExtFlag)
		str->cur_p += ColorLength;

	//ImageTxt
	Jbig2_ImageMarg(ImagePage->Image, ImageTxt, CombOp, TextRegion->RegionSegmentYlocation, TextRegion->RegionSegmentXlocation, 1, TextRegion->ColourExtFlag, ONE );

#if JBIG2_DEBUG05
	char	fname[256];
//	strcpy(fname, "ImageTxt_Dec000");	
//	fname[14]=0x30+(TxtCounter%10);
//	fname[13]=0x30+((TxtCounter/10) %10);
//	fname[12]=0x30+((TxtCounter/100) %10);
//	strcat(fname, ".bmp");
//	Jb2_Debug_Print( fname, ImageTxt, TxtCounter, TextRegion->ColourExtFlag );

	strcpy(fname, "ImageResiDec000");	
	fname[14]=0x30+(TxtCounter%10);
	fname[13]=0x30+((TxtCounter/10) %10);
	fname[12]=0x30+((TxtCounter/100) %10);
	strcat(fname, ".bmp");
	Jb2_Debug_Print( fname, ImagePage, 0, TextRegion->ColourExtFlag );
	TxtCounter++;

#endif

	delete	[] ImageTxt->Pdata;
	delete	ImageTxt;

	return	TRUE;
}

//7.4.5
struct ImageChain_s *HalftoneRegionSegmentDec( struct ImageChain_s *ImagePage, struct ImageChain_s *ImageHaf, struct ImageChain_s *ImagePat, struct StreamChain_s *str, struct HalftoneRegionSegment_s *HalfRegion, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 numPattern, uchar ColorExtFlag )
{
	struct	ImagePlane_s *tempDD;
	byte4	*Gray;
	byte4	ID;
	uchar	*D_;
	byte4	j;
	byte4	iii,jjj,kkk,col1step;
	byte4	tempDD2, tempDD3, tempDD4, tempDD5, tempDD6;
	byte4	numGray;
	char	ATX1,ATY1=-1,ATX2=-3,ATY2=-1,ATX3=2,ATY3=-2,ATX4=-2,ATY4=-2;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=6, ATX7=0, ATY7=0, ATX8=0, ATY8=0, ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;
	char	TpGDon=0, Template, ExtTemplate=0, MMR;
	uchar	tempD;
#if JBIG2_DEBUG00
	void	*temptempD;
#endif
	//RegionSegmentInformationField
	HalfRegion->RegionSegmentBitmapWidth = Ref_4Byte(str);
	HalfRegion->RegionSegmentBitmapHeight = Ref_4Byte(str);
	HalfRegion->RegionSegmentXlocation = Ref_4Byte(str);
	HalfRegion->RegionSegmentYlocation = Ref_4Byte(str);
	HalfRegion->ExternalCombinationOperator = Ref_1Byte(str) & 7;


	//PatternDictionaryFlags
	tempD = Ref_1Byte(str);
	MMR         = tempD &1;
	Template    = (tempD &6)>>1;
	HalfRegion->EnableSkip = (tempD&8)>>3;
	HalfRegion->CombOp      = (tempD&0x70)>>4;
	HalfRegion->HDefPixel   = (tempD&0x80)>>7;

	HalfRegion->HGW = Ref_4Byte(str);
	HalfRegion->HGH = Ref_4Byte(str);
	HalfRegion->HGX = Ref_4Byte(str);
	HalfRegion->HGY = Ref_4Byte(str);
	HalfRegion->HRX = Ref_2Byte(str);
	HalfRegion->HRY = Ref_2Byte(str);

#if JBIG2_DEBUG00
	printf("[HalftoneRegionSegmentDec] numPattern=%d \n", numPattern);
#endif
	j=0;
	while( mask1[j]<(unsigned)numPattern){
		j++;
	}
	numPattern = j-1;
#if JBIG2_DEBUG00
	printf("[HalftoneRegionSegmentDec] 2nd numPattern=%d \n", numPattern); 
#endif
	tempDD = ImagePlaneCreate(numPattern);
#if JBIG2_DEBUG00
	printf("[HalftoneRegionSegmentDec] ImagePlaneCreate complette \n"); 
#endif
	numGray = HalfRegion->HGW*HalfRegion->HGH;
	Gray = new byte4 [ numGray ];
	memset(Gray, 0, sizeof(byte4)*numGray );
	if(MMR){
		for(j=0;j<numPattern;j++)
			tempDD->Image[j] = T4T6DecMain( NULL, str, T6, MSBFirst, HalfRegion->HGW, HalfRegion->HGH );
	}
	else{
		if(Template<=1)	ATX1=3;
		else			ATX1=2;
		InitMQ_Codec( codec, str, codec->numCX, DEC, str->buf_length, JBIG2 );
		for(j=0;j<numPattern;j++)
			tempDD->Image[j] = MQ_DecImage( HalfRegion->HGW, HalfRegion->HGH, codec, str, str->buf_length, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
	}
	str = ByteStuffOutJXR(str);

	for(jjj=0;jjj<(signed)HalfRegion->HGH;jjj++){
		for(iii=0;iii<(signed)HalfRegion->HGW;iii++){
			kkk=0;
			D_ = (uchar *)tempDD->Image[kkk]->data;
			col1step=tempDD->Image[kkk]->col1step;
			tempDD2 = D_[jjj*col1step+iii];
			for(kkk=1;kkk<numPattern;kkk++){
				D_ = (uchar *)tempDD->Image[kkk]->data;
				col1step=tempDD->Image[kkk]->col1step;
				tempDD3 = tempDD2;
				tempDD4 = tempDD3&1;
				tempDD5 = tempDD4 ^ D_[jjj*col1step+iii];
				tempDD6 = tempDD2<<1;
				tempDD2 = tempDD6 + tempDD5;
			}
			Gray[jjj*HalfRegion->HGW+iii]=tempDD2;
		}
	}
	ImageHaf = ImageChainCreate(ImageHaf);
	ImageHaf->Image=NULL;
	ImageHaf->Image = ImageCreate( ImageHaf->Image, HalfRegion->RegionSegmentBitmapWidth, HalfRegion->RegionSegmentBitmapHeight, 0, HalfRegion->RegionSegmentBitmapWidth, 0, HalfRegion->RegionSegmentBitmapHeight, CHAR); 
#if JBIG2_DEBUG00
	printf("[HalftoneRegionSegmentDec] ImageHaf->Image Create complette \n");
#endif
	for( jjj=0 ; jjj<(signed)HalfRegion->HGH ; jjj++ ){
		for( iii=0 ; iii<(signed)HalfRegion->HGW ; iii++ ){
			ID = Gray[jjj*HalfRegion->HGW+iii];
			ImagePat = ImageChainSearch( ImagePat, ID);
			Jbig2_ImageMarg(ImageHaf->Image, ImagePat->Image, HalfRegion->CombOp, jjj*4, iii*4, Jb2_TopLeft, 0, NULL);
		}
	}
	Jbig2_ImageMarg(ImagePage->Image, ImageHaf->Image, HalfRegion->ExternalCombinationOperator, HalfRegion->RegionSegmentYlocation, HalfRegion->RegionSegmentXlocation, 1, 0, NULL );

	ImagePlaneDestroy( tempDD );
	delete	[] Gray;
	return	ImageHaf;
}

//7.4.4 
struct ImageChain_s *PatternDictionarySegmentDec( struct ImageChain_s *ImagePat, struct StreamChain_s *str, struct PatternDictionarySegment_s *PatternDic, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec )
{
	struct	Image_s *Image;
	uchar	*Ds_TS, *Ds_, *Dt_TS, *Dt_;
	byte4	iii, jjj, kkk, numPattern;
	byte4	MQ_Eaddr;
	byte4	Width;
	char	TpGDon=0, ATX1, ATY1=0, ATX2=-3, ATY2=-1, ATX3=2, ATY3=-2, ATX4=-2, ATY4=-2;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0, ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;
	char	Template, ExtTemplate=0;
	uchar	tempD;

	MQ_Eaddr = str->cur_p+PatternDic->SegmentDataLength;

	//PatternDictionaryFlags
	tempD = Ref_1Byte(str);
	PatternDic->HdMMR = tempD &1;
	Template = (tempD &6)>>1;

	PatternDic->HDPW = Ref_1Byte(str);
	PatternDic->HDPH = Ref_1Byte(str);
	ATX1 = -1*PatternDic->HDPW;

	PatternDic->GrayMax = Ref_4Byte(str);
	numPattern = PatternDic->GrayMax+1;
	PatternDic->numPattern = numPattern;
	Width = numPattern*PatternDic->HDPW;

	if(PatternDic->HdMMR)
		Image = T4T6DecMain( NULL, str, T6, MSBFirst, Width, PatternDic->HDPH);
	else{
		InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
		Image = MQ_DecImage( Width, PatternDic->HDPH, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
	}
	str = ByteStuffOutJXR(str);


	for( kkk=0 ; kkk<numPattern ; kkk++){
		ImagePat = ImageChainCreate(ImagePat);
		ImagePat->Image = ImageCreate(NULL, PatternDic->HDPW, PatternDic->HDPH, 0, PatternDic->HDPW, 0, PatternDic->HDPH, CHAR );
		ImagePat->Image->MaxValue=-1;
		Ds_TS = (uchar *)Image->data;
		Dt_TS = (uchar *)ImagePat->Image->data;
		for(jjj=0;jjj<PatternDic->HDPH ; jjj++, Ds_TS += Image->col1step, Dt_TS+=ImagePat->Image->col1step ){
			Ds_ = &Ds_TS[kkk*PatternDic->HDPW];
			Dt_ = Dt_TS;
			for( iii=0 ; iii<PatternDic->HDPW ; iii++, ++Dt_, ++Ds_ )
				*Dt_ = *Ds_;
		}
	}
	delete [] Image->Pdata;
	delete Image;

	return	ImagePat;
}


//6.2   GenericRegionSegment
struct ImageChain_s *GenericRegionSegmentDec( struct ImageChain_s *ImagePage, struct ImageChain_s *ImageGen, struct StreamChain_s *str, struct GenericRegionSegment_s *GenericRegion, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, uchar ColorExtFlag)
{
	byte4	width, height, Xlocation, Ylocation;
	byte4	*ONE=NULL;
	byte4	MQ_Eaddr;
	char	ATX1=0, ATY1=0, ATX2=0, ATY2=0, ATX3=0, ATY3=0, ATX4=0, ATY4=0;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0;
	char	ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;
	uchar	TpGDon, ExtTemplate, Template, MMR, tempD, Operator;
	uchar	Flag;

	if(ColorExtFlag){
		ONE = new byte4 [3];
		ONE[0]=ONE[1]=ONE[2]=1;
	}
	MQ_Eaddr = str->cur_p+GenericRegion->SegmentDataLength;

	//RegionSegmentInformationField
	width = Ref_4Byte(str);
	height = Ref_4Byte(str);
	Xlocation = Ref_4Byte(str);
	Ylocation = Ref_4Byte(str);
	Flag = Ref_1Byte(str);
	Operator=Flag&7;
	GenericRegion->ColourExtFlag = (Flag&8)>>3;

	//GenericRegionSegmentFlags
	tempD = Ref_1Byte(str);
	MMR = tempD &1;
	Template = (tempD&6)>>1;
	ExtTemplate = (tempD&0x10)>>4;
	TpGDon = (tempD&8)>>3;
	if(!MMR){
		if(!ExtTemplate){
			if(!Template){
				ATX1=(signed)Ref_1Byte(str);
				ATY1=(signed)Ref_1Byte(str);
				ATX2=(signed)Ref_1Byte(str);
				ATY2=(signed)Ref_1Byte(str);
				ATX3=(signed)Ref_1Byte(str);
				ATY3=(signed)Ref_1Byte(str);
				ATX4=(signed)Ref_1Byte(str);
				ATY4=(signed)Ref_1Byte(str);
			}
			else{
				ATX1=(signed)Ref_1Byte(str);
				ATY1=(signed)Ref_1Byte(str);
			}
		}
		else{
			ATX1=(signed)Ref_1Byte(str);
			ATY1=(signed)Ref_1Byte(str);
			ATX2=(signed)Ref_1Byte(str);
			ATY2=(signed)Ref_1Byte(str);
			ATX3=(signed)Ref_1Byte(str);
			ATY3=(signed)Ref_1Byte(str);
			ATX4=(signed)Ref_1Byte(str);
			ATY4=(signed)Ref_1Byte(str);
			ATX5=(signed)Ref_1Byte(str);
			ATY5=(signed)Ref_1Byte(str);
			ATX6=(signed)Ref_1Byte(str);
			ATY6=(signed)Ref_1Byte(str);
			ATX7=(signed)Ref_1Byte(str);
			ATY7=(signed)Ref_1Byte(str);
			ATX8=(signed)Ref_1Byte(str);
			ATY8=(signed)Ref_1Byte(str);
			ATX9=(signed)Ref_1Byte(str);
			ATY9=(signed)Ref_1Byte(str);
			ATX10=(signed)Ref_1Byte(str);
			ATY10=(signed)Ref_1Byte(str);
			ATX11=(signed)Ref_1Byte(str);
			ATY11=(signed)Ref_1Byte(str);
			ATX12=(signed)Ref_1Byte(str);
			ATY12=(signed)Ref_1Byte(str);
		}
	}

	ImageGen = ImageChainCreate(ImageGen);
	if(MMR){//TpGDon function is inhibited in MMR.So "TpGDon" is not pushed in T4T6DecMain.
		ImageGen->Image = T4T6DecMain( NULL, str, T6, MSBFirst, width, height );
		ImageGen->Image->MaxValue=-1;
	}
	else{
		InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
		ImageGen->Image = MQ_DecImage( width, height, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
		ImageGen->Image->MaxValue=-1;
	}
	str = ByteStuffOutJXR(str);

#if JBIG2_DEBUG03
	char	fname[256];
	struct	Image_s *rImage=NULL;
	strcpy(fname, "ImageGen000");
	fname[10]=0x30+GenCounter;
	strcat(fname, ".bmp");
	Jb2_Debug_Print( fname, ImageGen, GenCounter, 0 );
	GenCounter++;
#endif
	if(!ColorExtFlag)
		Jbig2_ImageMarg(ImagePage->Image, ImageGen->Image, Operator, Ylocation, Xlocation, 1, ColorExtFlag, ONE);

	return	ImageGen;
}

//6.3   GenericRefinementRegionSegment
struct ImageChain_s *GenericRefinementRegionDec( struct ImageChain_s *ImagePage, struct ImageChain_s *ImageGen, struct StreamChain_s *str, struct GenericRegionSegment_s *GenericRegion, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec)
{
	byte4	width, height, Xlocation, Ylocation;
	byte4	MQ_Eaddr;
	char	rATX1=0, rATY1=0, rATX2=0, rATY2=0;
	char	ATX1=0, ATY1=0, ATX2=0, ATY2=0, ATX3=0, ATY3=0, ATX4=0, ATY4=0;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0;
	char	ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;
	uchar	TpGDon, Template, ExtTemplate, tempD, Operator;

	MQ_Eaddr = str->cur_p+GenericRegion->SegmentDataLength;

	//RegionSegmentInformationField
	width = Ref_4Byte(str);
	height = Ref_4Byte(str);
	Xlocation = Ref_4Byte(str);
	Ylocation = Ref_4Byte(str);
	Operator = Ref_1Byte(str) & 7;

	//GenericRegionSegmentFlags
	tempD = Ref_1Byte(str);
	Template = (tempD&1);
	ExtTemplate = (tempD&0x10)>>4;
	TpGDon = (tempD&2)>>1;

	if(!Template){
		rATX1=(signed)Ref_1Byte(str);
		rATY1=(signed)Ref_1Byte(str);
		rATX2=(signed)Ref_1Byte(str);
		rATY2=(signed)Ref_1Byte(str);
	}

	ImageGen = ImageChainCreate(ImageGen);
	InitMQ_Codec( codec, str, codec->numCX, DEC, MQ_Eaddr, JBIG2 );
	ImageGen->Image = MQ_DecImage( width, height, codec, str, MQ_Eaddr, TpGDon, Template, ExtTemplate, ATX1, ATY1, ATX2, ATY2, ATX3, ATY3, ATX4, ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12);
	ImageGen->Image->MaxValue=-1;
	str = ByteStuffOutJXR(str);

	Jbig2_ImageMarg(ImagePage->Image, ImageGen->Image, Operator, Ylocation, Xlocation, 1, 0, NULL );

	return	ImageGen;
}

void EndOfPageSegmentDec( struct ImageChain_s *ImagePage, byte4 PageNumber )
{
#if JBIG2_DEBUG04
	char	fname[256];
	struct	Image_s *rImage=NULL;
	strcpy(fname, "Page00");		fname[5]=0x30+PageNumber;	strcat(fname, ".bmp");
	Jb2_Debug_Print( fname, ImagePage, PageNumber, 0 );
#endif
}

struct ImageChain_s *PageInformationSegmentDec( struct ImageChain_s *ImagePage, struct PageInformationSegment_s *PageInfo, struct StreamChain_s *str, uchar *ColorExtFlag )
{
	ubyte2	PageStripingInformation;
	uchar	PageSegmentFlags;

	PageInfo->PageBitmapWidth = Ref_4Byte(str);
	PageInfo->PageBitmapHeight = Ref_4Byte(str);
	PageInfo->PageXResolution = Ref_4Byte(str);
	PageInfo->PageYResolution = Ref_4Byte(str);
	
	PageSegmentFlags = Ref_1Byte(str);
	PageInfo->PageEventuallyLossless            = PageSegmentFlags&1;
	PageInfo->PageMightContainRefinements       = (PageSegmentFlags&2)>>1;
	PageInfo->PageDefaultPixelValue             = (PageSegmentFlags&4)>>2;
	PageInfo->PageDefaultCombinationOperator    = (PageSegmentFlags&0x18)>>3;
	PageInfo->PageRequiersAuxllaryBuffers       = (PageSegmentFlags&0x20)>>5;
	PageInfo->PageCombinationOperatorOverRidden = (PageSegmentFlags&0x40)>>6;
	PageInfo->ColorExtFlag = (PageSegmentFlags&0x80)>>7;
	*ColorExtFlag = PageInfo->ColorExtFlag;
	PageStripingInformation = Ref_2Byte(str);
	PageInfo->PageStriped                       = (PageStripingInformation)>>15;
	PageInfo->MaximumStripeSize                 = PageStripingInformation&0x7fff;

	ImagePage = ImageChainCreate(ImagePage);
	if(PageInfo->ColorExtFlag){
		ImagePage->Image = ImageCreate( NULL, PageInfo->PageBitmapWidth*3,  PageInfo->PageBitmapHeight*3, 0,  PageInfo->PageBitmapWidth, 0, PageInfo->PageBitmapHeight, CHAR);
		ImagePage->Image->row1step=3;
		ImagePage->Image->MaxValue=255;
	}
	else{
		ImagePage->Image = ImageCreate( NULL, PageInfo->PageBitmapWidth,  PageInfo->PageBitmapHeight, 0,  PageInfo->PageBitmapWidth, 0, PageInfo->PageBitmapHeight, CHAR);
	}
	return	ImagePage;
}


void ColourPaletteSegmentDec( struct ColourPaletteSegment_s *ColourPalette, struct StreamChain_s *str )
{
	ubyte2	PageStripingInformation=0;
	uchar	ColourPaletteFlags;

	
	ColourPaletteFlags = Ref_1Byte(str);
	ColourPalette->FlagfieldContinuesToNextByte = ColourPaletteFlags&1;
	ColourPalette->ColourSpace                  = (ColourPaletteFlags&0x1e)>>1;

	ColourPalette->NumCompts = Ref_1Byte(str);
	ColourPalette->ComptsLength = Ref_1Byte(str);
	ColourPalette->ComptsValue  = Ref_4Byte(str);
}
