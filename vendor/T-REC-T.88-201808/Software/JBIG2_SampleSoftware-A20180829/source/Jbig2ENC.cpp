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
//#include	<math.h>
#include    <string.h>
#include	"ImageUtil.h"
#include	"Jb2Common.h"
#include	"T4T6codec.h"
#include	"Jb2_MQLapper.h"
#include	"Jb2_T4T6Lapper.h"
#include	"T45_codec.h"
#include	"Jb2_Debug.h"




struct StreamChain_s *JBIG2_EncMain( struct StreamChain_s *str, struct Jbig2Parameter_s *Jb2Param, struct ImageChain_s *ImagePage )
{
	struct	Jb2SegmentHeader_s *Seg;
	struct	Jb2HuffmanTable_s *Huff;
	struct	mqcodec_s *codec;
	byte4	numPage=1;
	byte4	i;

	//FileHeader
	for(i=0 ; i<8 ; i++ )
		str = Stream1ByteWrite(str, JB2_FILE_HEADER_ID[i], str->buf_length );

	str = Stream1ByteWrite(str, 1, str->buf_length );
	str = Stream4ByteWrite(str, numPage, str->buf_length, BIG_ENDIAN );

	Huff = CreateHuffmanTable( ENC );
	codec = new struct mqcodec_s;
	codec->numCX = Number_CX;
	codec->index = new uchar [codec->numCX];

	Seg = SegmentCreate( Jb2Param );
	str = SegmentEncode( str, Jb2Param, Seg, Huff, codec, ImagePage );


	return	str;
}

struct StreamChain_s *SegmentEncode( struct StreamChain_s *str, struct Jbig2Parameter_s *Jb2Param, struct Jb2SegmentHeader_s *Seg, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, struct ImageChain_s *ImagePage )
{
	byte4	j;
	byte4	Saddr, Eaddr, DataPartLength;
	byte4	SymbolCount=0;
	uchar	SegmentHeaderFlags;
	char	PageFlag=1;
	//struct	PatternDictionarySegment_s *PatternDic;
	struct	Jb2_ImageChain_s *ImageSym=NULL;
	struct	ImageChain_s *ImageTxt=NULL, *ImageHaf=NULL, *ImagePat=NULL, *ImageGen=NULL;

	ImageSym = Jb2Param->ImageSym;
	ImageGen = Jb2Param->ImageGen;
	//PatternDic = Jb2Param->PatternDic;

	for(j=0 ; j<Jb2Param->NumberOfSegments ; j++){
		str = Stream4ByteWrite(str, Seg[j].SegNo, str->buf_length, BIG_ENDIAN);//Segment number 7.2
		SegmentHeaderFlags = Seg[j].PageAssociationSize | Seg[j].SegmentType;//Segment header flags 7.2.3
		str = Stream1ByteWrite(str, SegmentHeaderFlags, str->buf_length);
		str = Stream1ByteWrite(str, 0, str->buf_length);//Referred-to segment count and retention flags 7.2.4 
		if(Seg[j].PageAssociationSize)
			str = Stream4ByteWrite( str, Seg[j].PageAssociate, str->buf_length, BIG_ENDIAN);
		else
			str = Stream1ByteWrite( str, (uchar)Seg[j].PageAssociate, str->buf_length );		
		//Segment Data Length
		str = Stream4ByteWrite( str, Seg[j].SegmentDataLength, str->buf_length, BIG_ENDIAN);
		switch(Seg[j].SegmentType){
		case SYMBOL_DICTIONARY:
			Saddr = str->cur_p;
			str = SymbolDictionarySegmentEnc( Seg[j].SymbolDic, ImageSym, str, Huff, codec, &SymbolCount, PageFlag );
			Eaddr = str->cur_p;
			DataPartLength = Eaddr-Saddr;
			str->cur_p=Saddr-4;
			str = Stream4ByteWrite(str, DataPartLength, str->buf_length, BIG_ENDIAN );
			str->cur_p = Eaddr;
			break;
		case INTERMEDIATE_TEXT_REGION:
		case IMMEDIATE_TEXT_REGION:
		case IMMEDIATE_LOSLESS_TEXT_REGION:
			Saddr = str->cur_p;
			str = TextRegionSegmentEnc( str, Seg[j].TextRegion, ImageSym, ImagePage, Huff, codec, SymbolCount );
			Eaddr = str->cur_p;
			DataPartLength = Eaddr-Saddr;
			str->cur_p=Saddr-4;
			str = Stream4ByteWrite(str, DataPartLength, str->buf_length, BIG_ENDIAN );
			str->cur_p = Eaddr;
			break;
		case PATTERN_DICTIONARY:
//			ImagePat = PatternDictionarySegmentEnc( ImagePat, str, &Seg[j], PatternDic, Huff/*, codec*/);
			break;
		case INTERMIDIATE_HALFTONE_REGION:
			break;
		case IMMEDIATE_HALFTONE_REGION:
			break;
		case IMMEDIATE_LOSLESS_HALFTONE_REGION:
//			ImageHaf = ImmediateLosslessHalftoneRegionSegmentEncc( ImagePage, ImageHaf, ImagePat, str, &Seg[j], Huff, PatternDic->numPattern/*, codec*/);
			break;
		case INTERMEDIATE_GENERIC_REGION:
		case IMMEDIATE_GENERIC_REGION:
		case IMMEDIATE_LOSLESS_GENERIC_REGION:
			Saddr = str->cur_p;
			str = ImmediateLosslessGenericRegionSegmentEnc( Seg[j].GenericRegion, str, ImagePage, Huff, codec );
			Eaddr = str->cur_p;
			DataPartLength = Eaddr-Saddr;
			str->cur_p=Saddr-4;
			str = Stream4ByteWrite(str, DataPartLength, str->buf_length, BIG_ENDIAN );
			str->cur_p = Eaddr;
			break;
		case INTERMEDIATE_GENERIC_REFINMENT_REGION:
			break;
		case IMMEDIATE_GENERIC_REFINMENT_REGION:
			break;
		case IMMEDIATE_LOSLESS_GENERIC_REFINMENT_REGION:
			break;
		case PAGE_INFORMATION:
			Saddr = str->cur_p;
			str = PageInformationSegmentEnc( Jb2Param->PageInfo, str );
			Eaddr = str->cur_p;
			DataPartLength = Eaddr-Saddr;
			str->cur_p=Saddr-4;
			str = Stream4ByteWrite(str, DataPartLength, str->buf_length, BIG_ENDIAN );
			str->cur_p = Eaddr;
			PageFlag=0;
			break;
		case END_OF_PAGE:
			Saddr = str->cur_p;
			str = EndOfPageSegmentEnc( Jb2Param, str );
			Eaddr = str->cur_p;
			DataPartLength = Eaddr-Saddr;
			str->cur_p=Saddr-4;
			str = Stream4ByteWrite(str, DataPartLength, str->buf_length, BIG_ENDIAN );
			str->cur_p = Eaddr;
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
	return	str;
}

#if 0
struct StreamChain_s *SymbolAggregate_MQEnc( struct Image_s *AggImage, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, uchar RefinementAggregate, uchar Ri, uchar StripT0, byte4 deltaT, uchar RefCorner, uchar CombOp, uchar DsOffset, byte4 SbNumInstances, byte4 numCode, struct mqcodec_s *codec, ubyte4 numSymbol, uchar Template, char RATX1, char RATY1, char RATX2, char RATY2)
{
	struct	Image_s *RefImage;
	byte4	nInstances, StripT;
	byte4	/*deltaT,*/ deltaS;
	byte4	Cur_S, Cur_T;
	byte4	SymbolCodeLength;
	byte4	ID, RDw=0, RDh=0, RDx, RDy, RefDx, RefDy;
	uchar	Flag=1, FirstS=1, TpGDon=0;

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

	Flag=1;
	//Initial StripT value
	str = MQ_EncInteger(1, str, codec, IADT );//deltaT=1 fixed
	StripT = StripT0 * deltaT * (-1);
	nInstances=0;
	do{
		//DeltaT
		str = MQ_EncInteger(1, str, codec, IADT );
		StripT += (deltaT * StripT0);
		while( 1 ){
			//Instance S
			if(FirstS){
				str = MQ_EncInteger(deltaS, str, codec, IAFS );
				//deltaS = MQ_DecInteger( str, codec, IAFS, MQ_Eaddr, &Flag );
				Cur_S = deltaS;
				FirstS=0;
			}
			else{
				str = MQ_EncInteger(deltaS, str, codec, IADS );
				//deltaS = MQ_DecInteger( str, codec, IADS, MQ_Eaddr, &Flag );
				if(!Flag)
					break;
				Cur_S = Cur_S + deltaS + DsOffset;
			}

			//Instance T
			if((!RefinementAggregate) && (StripT0!=1)){
				str = MQ_EncInteger(deltaT, str, codec, IAIT );
				//deltaT = MQ_DecInteger( str, codec, IAIT, MQ_Eaddr, &Flag );
				Cur_T = deltaT + StripT0;
			}

			//SymbolID
			//ID = MQ_DecIntegerIAID(str, codec, MQ_Eaddr, SymbolCodeLength, IAID );
			if( RefinementAggregate ){
				//Ri = MQ_DecInteger( str, codec, IARI, MQ_Eaddr, &Flag );
				str = MQ_EncInteger( Ri, str, codec, IARI );
				if(Ri){
					str = MQ_EncIntegerIAID( ImageSym->RefID, str, codec, SymbolCodeLength, IAID );
					ImageSym = Jb2_ImageChainSearch( ImageSym, ID );
					//RefImage = ImageSym->Image;
					//RDw = MQ_DecInteger( str, codec, IARDW, MQ_Eaddr, &Flag );
					//RDh = MQ_DecInteger( str, codec, IARDH, MQ_Eaddr, &Flag );
					//RDx = MQ_DecInteger( str, codec, IARDX, MQ_Eaddr, &Flag );
					//RDy = MQ_DecInteger( str, codec, IARDY, MQ_Eaddr, &Flag );
					str = MQ_EncInteger( RDw, str, codec, IARDW );
					str = MQ_EncInteger( RDh, str, codec, IARDH );
					str = MQ_EncInteger( RDx, str, codec, IARDX );
					str = MQ_EncInteger( RDy, str, codec, IARDY );
					RefDx = floor2(RDw, 2) + RDx;
					RefDy = floor2(RDh, 2) + RDy;
					RDw += (ImageSym->Image->tbx1-ImageSym->Image->tbx0);
					RDh += (ImageSym->Image->tby1-ImageSym->Image->tby0);
					/*RefDy=0;*/ /*RefDx=-1;*/
					//RefImage = MQ_RefinementDecImage( ImageSym->Image, RDw, RDh, RefDx, RefDy, codec, str, MQ_Eaddr, TpGDon, Template, RATX1, RATY1, RATX2, RATY2 );
					str = MQ_RefinementEncImage( RefImage, ImageSym->Image, RefDx, RefDy, codec, str, TpGDon, Template, RATX1, RATY1, RATX2, RATY2 );
					//Jbig2_ImageMarg( AggImage, RefImage, CombOp, Cur_T, Cur_S, RefCorner);
					Cur_S += (RefImage->tbx1-RefImage->tbx0-1);
				}
				else{
					str = MQ_EncIntegerIAID( ImageSym->ID, str, codec, SymbolCodeLength, IAID );
					ImageSym = Jb2_ImageChainSearch( ImageSym, ID);
					Jbig2_ImageMarg( AggImage, ImageSym->Image, CombOp, Cur_T, Cur_S, RefCorner);
					Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
				}
			}
			else{
				str = MQ_EncIntegerIAID( ImageSym->ID, str, codec, SymbolCodeLength, IAID );
				ImageSym = Jb2_ImageChainSearch( ImageSym, ID);
				Jbig2_ImageMarg( AggImage, ImageSym->Image, CombOp, Cur_T, Cur_S, RefCorner);
				Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
			}
			nInstances++;
		};
	} while(nInstances<SbNumInstances);
	return	str;
}
#endif

//7.4.2
struct StreamChain_s *SymbolDictionarySegmentEnc( struct SymbolDictionarySegment_s *SymbolDic, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 *SymbolCount, char PageFlag)
{
	struct	StreamChain_s *str2=NULL;
	struct	Image_s *Image, *RefImage;
	byte4	numHeight;
	byte4	DeltaWidth, TotalWidth, xbyte, **Width, *numWidth;
	byte4	DeltaHeight, *Height;
	byte4	i, j, jjj, kkk, SymbolCodeLength;
	byte4	RefID, RefDx, RefDy;
	ubyte2	SymbolDicFlags;
	uchar	H_No_Huff,W_No_Huff,B_No_Huff;
	uchar	Flag=0, TpGDon=0, ExtTemplate=0;
	uchar	*tempD;
	char	ATX5=0, ATY5=0, ATX6=0, ATY6=0, ATX7=0, ATY7=0, ATX8=0, ATY8=0, ATX9=0, ATY9=0, ATX10=0, ATY10=0, ATX11=0, ATY11=0, ATX12=0, ATY12=0;

	numHeight = SymbolDic->numHeight;
	Height    = SymbolDic->Height;
	numWidth  = SymbolDic->numWidth;
	Width     = SymbolDic->Width;
	//SymbolDicFlags Write
	SymbolDicFlags =  (SymbolDic->Huff & 1);
	SymbolDicFlags |= ((SymbolDic->RefinementAggregate&0x1)<<1);
	SymbolDicFlags |= ((SymbolDic->Huff_DH_Selection&0x3)<<2);
	SymbolDicFlags |= ((SymbolDic->Huff_DW_Selection&0x3)<<4);
	SymbolDicFlags |= ((SymbolDic->Huff_BmSize_Selection&0x1)<<6);
	SymbolDicFlags |= ((SymbolDic->Huff_Agginst_Selection&0x1)<<7);
	SymbolDicFlags |= ((SymbolDic->BitmapCodingContextUsed &0x1)<<8);
	SymbolDicFlags |= ((SymbolDic->BitmapCodingContextRetained&0x1)<<9);
	SymbolDicFlags |= ((SymbolDic->Template&0x3)<<10);
	SymbolDicFlags |= ((SymbolDic->RefTemplate&0x1)<<12);
	str = Stream2ByteWrite( str, SymbolDicFlags, str->buf_length, BIG_ENDIAN );

	if(!SymbolDic->Huff){
		//Symbol Dictionary AT flags
		if(!SymbolDic->Template){//Template==0
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATX1), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATY1), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATX2), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATY2), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATX3), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATY3), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATX4), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATY4), str->buf_length);
		}
		else{
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATX1), str->buf_length);
			str = Stream1ByteWrite( str, (uchar)(SymbolDic->ATY1), str->buf_length);
		}
	}

	//Refinement or Aggregate.
	if( SymbolDic->RefinementAggregate && (!SymbolDic->RefTemplate) ){
		//Symbol Dictionary Refinement AT flags
		str = Stream1ByteWrite( str, (uchar)(SymbolDic->RefATX1), str->buf_length);
		str = Stream1ByteWrite( str, (uchar)(SymbolDic->RefATY1), str->buf_length);
		str = Stream1ByteWrite( str, (uchar)(SymbolDic->RefATX2), str->buf_length);
		str = Stream1ByteWrite( str, (uchar)(SymbolDic->RefATY2), str->buf_length);
	}

	//Number of Exported Symbol
	str = Stream4ByteWrite( str, (ubyte4)(SymbolDic->SdNumExSyms), str->buf_length, BIG_ENDIAN );

	//
	str = Stream4ByteWrite( str, (ubyte4)(SymbolDic->SdNumNewSyms), str->buf_length, BIG_ENDIAN );
	if(!PageFlag)
		SymbolDic->numSymbol_Page += SymbolDic->SdNumNewSyms;
	else
		SymbolDic->numSymbol_File += SymbolDic->SdNumNewSyms;

	for( SymbolCodeLength=31 ; SymbolCodeLength>0 ; SymbolCodeLength--){
		if(	mask5[SymbolCodeLength]&SymbolDic->SdNumExSyms ){
			if( (mask6[SymbolCodeLength] & SymbolDic->SdNumExSyms) ){
				SymbolCodeLength++;
				break;
			}
			else
				break;
		}
	}



	if(SymbolDic->RefinementAggregate){
		if(SymbolDic->Huff){
		}
		else{
			InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
			for( kkk=0 ; kkk<numHeight ; kkk++ ){//HeightClass Loop
				if(kkk==0)	DeltaHeight = Height[0];
				else		DeltaHeight = Height[kkk]-Height[kkk-1];
				str = MQ_EncInteger( DeltaHeight, str, codec, IADH );//IADH
				for( jjj=0 ; jjj<numWidth[kkk] ;   ){
					if(jjj==0)	DeltaWidth = Width[kkk][0];
					else		DeltaWidth = Width[kkk][jjj] - Width[kkk][jjj-1];
					str = MQ_EncInteger( DeltaWidth, str, codec, IADW );//IADW
					str = MQ_EncInteger( SymbolDic->RefAggnInst, str, codec, IAAI );//IAAI
					//
					while(SymbolDic->RefAggnInst){
						ImageSym = Jb2_ImageChainSearch(ImageSym, (*SymbolCount) );
						(*SymbolCount)++;
						Image = ImageSym->Image;
						RefID = ImageSym->RefID;
						RefDx = ImageSym->RDx;
						RefDy = ImageSym->RDy;
						ImageSym = Jb2_ImageChainSearch( ImageSym, RefID ); 
						RefImage = ImageSym->Image;
						str = MQ_EncIntegerIAID( RefID, str, codec, SymbolCodeLength, IAID );	//IAID
						str = MQ_EncInteger( RefDx, str, codec, IARDX );	//IAID
						str = MQ_EncInteger( RefDy, str, codec, IARDY );	//IAID
						str = MQ_RefinementEncImage( RefImage, Image, RefDx, RefDy, codec, str,TpGDon, SymbolDic->RefTemplate, SymbolDic->RefATX1, SymbolDic->RefATY1, SymbolDic->RefATX2, SymbolDic->RefATY2 );
						SymbolDic->RefAggnInst--;
						jjj++;
					}
					if(jjj<numWidth[kkk]){
						ImageSym = Jb2_ImageChainSearch( ImageSym, (*SymbolCount) );
						SymbolCount++;
					}
				}
				//When RefAggnInst, number of symbol is fixed. So "OOB" should not occered. 
				//str = MQ_EncInteger( OOB, str, codec, IADW );//Width Loop End need OOB.
			}
			str = MQ_EncInteger( 0, str, codec, IAEX );//IAEX
			str = MQ_EncInteger( 0, str, codec, IAEX );//IAEX
			str = MQ_flush( codec, str );
		}
	}
	else{
		//HUFF
		if(SymbolDic->Huff){
			switch(SymbolDic->Huff_DW_Selection){
			case 0:	W_No_Huff=1;	break;//TableB2
			case 1:	W_No_Huff=2;	break;//TableB3
			case 3:	W_No_Huff=15;	break;
			default:				break;
			}
			switch(SymbolDic->Huff_DH_Selection){
			case 0:H_No_Huff=3;		break;//TableB4
			case 1:H_No_Huff=4;		break;//TableB5
			case 3:H_No_Huff=15;	break;//
			default:				break;
			}
			//SDHUFFBMSIZE
			if(!SymbolDic->Huff_BmSize_Selection)	B_No_Huff=0;
			else									B_No_Huff=15;

			for( kkk=0 ; kkk<numHeight ; kkk++ ){
				if(kkk==0)	DeltaHeight = Height[0];
				else		DeltaHeight = Height[kkk]-Height[kkk-1];
				str = JBIG2_HuffEnc( DeltaHeight, str, &Huff[H_No_Huff] );
					
				for(jjj=0 ; jjj<numWidth[kkk] ; jjj++){
					if(jjj==0)	DeltaWidth = Width[kkk][0];
					else		DeltaWidth = Width[kkk][jjj] - Width[kkk][jjj-1];
					str = JBIG2_HuffEnc( DeltaWidth, str, &Huff[W_No_Huff] );
				}
				str = JBIG2_HuffEnc( OOB, str, &Huff[W_No_Huff] );//

				if(SymbolDic->BmSize){//
					str = JBIG2_HuffEnc( SymbolDic->BmSize, str, &Huff[B_No_Huff] );//
					str = ByteStuffOutJXR( str );
					for( jjj=0 ; jjj<numWidth[kkk] ; jjj++, (*SymbolCount)++ ){
						ImageSym = Jb2_ImageChainSearch(ImageSym, (*SymbolCount) );
						Image = ImageSym->Image;
						str = T4T6Encmain( str, Image, 0, T6, 0);
					}
				}
				else{//
					str = JBIG2_HuffEnc( SymbolDic->BmSize, str, &Huff[B_No_Huff] );//
					str = ByteStuffOutJXR( str );
					TotalWidth=0;
					for( jjj=0 ; jjj<numWidth[kkk] ; jjj++ )
						TotalWidth+=Width[kkk][jjj];
					xbyte = ceil2(TotalWidth,8);
					tempD = new  uchar [TotalWidth*Height[kkk] ]; 
					for( j=0 ; j<Height[kkk] ; j++ ){
						for( i=0 ; i<TotalWidth ; i++ ){
						}
					}
				}
			}
			//
			str = Stream1ByteWrite(str, 0, str->buf_length);
			str = Stream1ByteWrite(str, 0, str->buf_length);
		}
		//No(Refinement & Aggregate) && Arithemetric
		else{
			InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
			for( kkk=0 ; kkk<numHeight ; kkk++ ){//HeightClass Loop
				if(kkk==0)	DeltaHeight = Height[0];
				else		DeltaHeight = Height[kkk]-Height[kkk-1];
				str = MQ_EncInteger( DeltaHeight, str, codec, IADH );//IADH
				for( jjj=0 ; jjj<numWidth[kkk] ; jjj++, (*SymbolCount)++ ){
					if(jjj==0)	DeltaWidth = Width[kkk][0];
					else		DeltaWidth = Width[kkk][jjj] - Width[kkk][jjj-1];
					str = MQ_EncInteger( DeltaWidth, str, codec, IADW );//IADW
					ImageSym = Jb2_ImageChainSearch( ImageSym, (*SymbolCount) );
					Image = ImageSym->Image;
					str = MQ_EncImage( Image, str, codec, TpGDon, SymbolDic->Template, ExtTemplate, SymbolDic->ATX1, SymbolDic->ATY1, SymbolDic->ATX2, SymbolDic->ATY2, SymbolDic->ATX3, SymbolDic->ATY3, SymbolDic->ATX4, SymbolDic->ATY4, ATX5, ATY5, ATX6, ATY6, ATX7, ATY7, ATX8, ATY8, ATX9, ATY9, ATX10, ATY10, ATX11, ATY11, ATX12, ATY12, 1 );
				}
				str = MQ_EncInteger( OOB, str, codec, IADW );//Width Loop End need OOB
			}
			str = MQ_EncInteger( 0, str, codec, IAEX );//IAEX
			str = MQ_EncInteger( 0, str, codec, IAEX );//IAEX
			str = MQ_flush( codec, str );
		}
	}
	return	str;
}

//7.4.3
struct StreamChain_s *TextRegionSegmentEnc( struct StreamChain_s *str, struct TextRegionSegment_s *TextRegion, struct Jb2_ImageChain_s *ImageSym, struct ImageChain_s *ImagePage, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, ubyte4 numSymbol )
{
	byte4	i, kkk, nInstances, StripT, StripT0, StripT1;
	byte4	numCode=35;
	byte4	deltaT, deltaS;
	byte4	Cur_S, Cur_T, RDw, RDh, RDx, RDy;
	byte4	SbSymbolCodeLength;
	byte4	*RunCode_L=NULL, *RunCode_V=NULL, *SymbolID_V=NULL, *SymbolID_L=NULL, *SymbolID_O=NULL, *RunCode_O=NULL;
	byte4	ID0, RefID;
	char	Aggregate=0, Refinement;
	uchar	Flag=1, FlagS, FirstS;
	ubyte2	tempD;
	uchar	No_Huff, TpGDon=0;
	byte4	*ID, *Lx, *Ly, *RefDx, *RefDy;
	byte4	TextImageCount=0;
	byte4	TempAddr;
	struct	Image_s *ImageTxt=NULL;

	ID = TextRegion->ID;
	Lx = TextRegion->Lx;
	Ly = TextRegion->Ly;
	RefDx = TextRegion->RefDx;
	RefDy = TextRegion->RefDy;

	//RegionSegment
	str = Stream4ByteWrite( str, TextRegion->RegionSegmentBitmapWidth, str->buf_length, BIG_ENDIAN );
	str = Stream4ByteWrite( str, TextRegion->RegionSegmentBitmapHeight, str->buf_length, BIG_ENDIAN );
	str = Stream4ByteWrite( str, TextRegion->RegionSegmentXlocation, str->buf_length, BIG_ENDIAN );
	str = Stream4ByteWrite( str, TextRegion->RegionSegmentYlocation, str->buf_length, BIG_ENDIAN );
	FlagS = ( (TextRegion->ColourExtFlag*8) + (TextRegion->ExternalCombinationOperator&7) );
	str = Stream1ByteWrite( str, FlagS, str->buf_length );

//	ImageTxt = ImageChainCreate(ImageTxt);
	ImageTxt = ImageCreate( ImageTxt,  TextRegion->RegionSegmentBitmapWidth, TextRegion->RegionSegmentBitmapHeight, 0, TextRegion->RegionSegmentBitmapWidth, 0, TextRegion->RegionSegmentBitmapHeight, CHAR);

	//TextRegionSegmentFlag
	tempD = TextRegion->Huff&1;
	tempD |= ((TextRegion->Refine & 1) <<1 );
	tempD |= ((TextRegion->LogSbStrips & 3) <<2 );
	tempD |= ((TextRegion->RefCorner & 3) <<4 );
	tempD |= ((TextRegion->Transposed & 1) <<6 );
	tempD |= ((TextRegion->SbCombOp & 3) <<7 );
	tempD |= ((TextRegion->SbDefPixel & 1) <<9 );
	tempD |= ((TextRegion->SbDsOffset & 0x1f) <<10 );
	tempD |= ((TextRegion->SbrTemplate & 1)<<15 );
	str = Stream2ByteWrite( str, (ubyte2)tempD, str->buf_length, BIG_ENDIAN );
	
	if(TextRegion->Huff){
		tempD = Ref_2Byte(str);
		tempD  =  (TextRegion->HuffFsSelection&3);
		tempD |= ((TextRegion->HuffDsSelection&0)<<2);
		tempD |= ((TextRegion->HuffDtSelection&0)<<4);
		tempD |= ((TextRegion->HuffRdWSelection&3)<<6);
		tempD |= ((TextRegion->HuffRdHSelection&3)<<8);
		tempD |= ((TextRegion->HuffRdXSelection&3)<<10);
		tempD |= ((TextRegion->HuffRdYSelection&3)<<12);
		tempD |= ((TextRegion->HuffRSizeSelection&1)<<14);
		str = Stream2ByteWrite( str, (ubyte2)tempD, str->buf_length, BIG_ENDIAN );
	}

	if(TextRegion->Refine && (!TextRegion->SbrTemplate) ){
		str = Stream1ByteWrite( str, TextRegion->RefATX1, str->buf_length );//=Ref_1Byte(str);
		str = Stream1ByteWrite( str, TextRegion->RefATY1, str->buf_length );//=Ref_1Byte(str);
		str = Stream1ByteWrite( str, TextRegion->RefATX2, str->buf_length );//=Ref_1Byte(str);
		str = Stream1ByteWrite( str, TextRegion->RefATY2, str->buf_length );//=Ref_1Byte(str);
	}

	str = Stream4ByteWrite( str, TextRegion->SbNumInstances, str->buf_length, BIG_ENDIAN );

	//SymbolID
	if(TextRegion->Huff){
		RunCode_L = new byte4 [numCode];
		RunCode_V = new byte4 [numCode];
		memset(RunCode_L, 0, sizeof(byte4)*numCode);
		memset(RunCode_V, 0, sizeof(byte4)*numCode);

		SymbolID_V = new byte4 [TextRegion->SbNumInstances];
		SymbolID_L = new byte4 [TextRegion->SbNumInstances];
		SymbolID_O = new byte4 [TextRegion->SbNumInstances];
		memset( SymbolID_V, 0, sizeof(byte4)*TextRegion->SbNumInstances );
		memset( SymbolID_L, 0, sizeof(byte4)*TextRegion->SbNumInstances );
		memset( SymbolID_O, -1, sizeof(byte4)*TextRegion->SbNumInstances );
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
	}
	else{

	}

	StripT0=(1<<TextRegion->LogSbStrips);	//StripT0 is Base Stripe value.
	StripT = StripT0;						//Initial Value is negative.
	Cur_T=0;
	if(TextRegion->Huff){
		//Initial StripT value
		switch(TextRegion->HuffDtSelection){// Huff_Dt Arith_IADT
		case 0:	No_Huff=10;	break;//TableB11
		case 1:	No_Huff=11;	break;//TableB12
		case 2: No_Huff=12;	break;//TableB13
		case 3:	No_Huff=15;	break;
		default:			break;
		}
		deltaT = 1;
		str = JBIG2_HuffEnc(deltaT, str, &Huff[No_Huff]);
		StripT1 = StripT0 * deltaT * (-1);
		StripT  = StripT1;
		nInstances=0;
		do{
			//DeltaT
			switch(TextRegion->HuffDtSelection){
			case 0:	No_Huff=10;	break;//TableB11
			case 1:	No_Huff=11;	break;//TableB12
			case 2: No_Huff=12;	break;//TableB13
			case 3:	No_Huff=15;	break;
			default:			break;
			}
			StripT = StripT + deltaT * StripT0;				//(j-2) "10" STRIPT = -4 + 2*TextRegion->LogSbStrips = 4//
			str = JBIG2_HuffEnc( deltaT, str, &Huff[No_Huff] );

			//Instance S
			switch(TextRegion->HuffFsSelection){
			case 0:	No_Huff= 5;	break;//TableB6
			case 1:	No_Huff= 6;	break;//TableB7
			case 3:	No_Huff=15;	break;
			default:			break;
			}
			deltaS = JBIG2_HuffDec(str, &Huff[No_Huff] );
			Cur_S = deltaS;						//(j-3) "00 0000000" 

			//Instance T
			deltaT = Ref_nBits(str, TextRegion->LogSbStrips);
			Cur_T = Cur_T + deltaT + StripT;	//(j-4) "01" Cur_T = STRIPT + 1 = 5//

			//SymbolID
			ID0 = JBIG2_ID_Dec( SymbolID_V, SymbolID_L, SymbolID_O, str );
			ImageSym = Jb2_ImageChainSearch( ImageSym, ID0);
			Jbig2_ImageMarg(ImageTxt, ImageSym->Image, TextRegion->SbCombOp, Cur_T, Cur_S, TextRegion->RefCorner, 0, NULL);

			Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
			nInstances++;
			while( 1 ){
				//Instance S
				switch(TextRegion->HuffDsSelection){
				case 0:	No_Huff= 7;	break;//TableB8
				case 1:	No_Huff= 8;	break;//TableB9
				case 2: No_Huff= 9;	break;//TableB10
				case 3:	No_Huff=15;	break;
				default:			break;
				}
				deltaS = JBIG2_HuffDec(str, &Huff[No_Huff] );
				if(deltaS==OOB)
					break;
				Cur_S = Cur_S + deltaS + TextRegion->SbDsOffset;

				//Instance T
				deltaT = Ref_nBits(str, TextRegion->LogSbStrips);
				Cur_T = deltaT + StripT0;	//(j-4) "01" Cur_T = STRIPT + 1 = 5//

				//SymbolID
				ID0 = JBIG2_ID_Dec( SymbolID_V, SymbolID_L, SymbolID_O, str );
				ImageSym = Jb2_ImageChainSearch( ImageSym, ID0);
				Jbig2_ImageMarg(ImageTxt, ImageSym->Image, TextRegion->SbCombOp, Cur_T, Cur_S, TextRegion->RefCorner, 0, NULL );
				Cur_S += (ImageSym->Image->tbx1-ImageSym->Image->tbx0-1);
				nInstances++;
			} ;
		} while(nInstances<TextRegion->SbNumInstances);
		str = ByteStuffOutJXR(str);
	}
	else{//ArithMetric
		for( SbSymbolCodeLength=31 ; SbSymbolCodeLength>0 ; SbSymbolCodeLength--){
			if(	mask5[SbSymbolCodeLength]&numSymbol ){
				if( (mask6[SbSymbolCodeLength] & numSymbol) ){
					SbSymbolCodeLength++;
					break;
				}
				else
					break;
			}
		}

		InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
		//Initial StripT value
		StripT0=(1<<TextRegion->LogSbStrips); //StripT0 is Base Stripe value.
		deltaT	 = 1; 
		str = MQ_EncInteger( deltaT, str, codec, IADT );//OK
		StripT = StripT0 * deltaT * (-1);
		nInstances=0;
		do{
			FirstS=1;
			//DeltaT
			deltaT = 2;
			str = MQ_EncInteger( deltaT, str, codec, IADT );//OK
			StripT = StripT + deltaT * StripT0;
			if(TextRegion->Refine){
				for(kkk=0 ; kkk<TextRegion->SbNumInstances ; kkk++, nInstances++){
					//Instance S
					if(FirstS){
						deltaS=Lx[kkk];
						str  = MQ_EncInteger( deltaS, str, codec, IAFS );//OK
						Cur_S = deltaS;	
						FirstS=0;
					}
					else{
						deltaS = Lx[kkk] - Cur_S;
						str = MQ_EncInteger( deltaS, str, codec, IADS );
					}
					RefID = ID[kkk]&0x7fffffff;
					Refinement = (ID[kkk]&0x80000000) ? 1:0;
					str = MQ_EncIntegerIAID( RefID, str, codec, SbSymbolCodeLength, IAID );
					str = MQ_EncInteger( Refinement, str, codec, IARI);
					if(Refinement){
						ImageSym = Jb2_ImageChainSearch( ImageSym, RefID);
						TextRegion->ImageSymT = Jb2_ImageChainSearch(TextRegion->ImageSymT, TextImageCount );
						RDw = TextRegion->ImageSymT->Image->width  - ImageSym->Image->width;
						RDh = TextRegion->ImageSymT->Image->height - ImageSym->Image->height;
						RDx = RefDx[kkk] - floor2( RDw, 2 );
						RDy = RefDy[kkk] - floor2( RDh, 2 );
						str = MQ_EncInteger( RDw, str, codec, IARDW );
						str = MQ_EncInteger( RDh, str, codec, IARDH );
						str = MQ_EncInteger( RDx, str, codec, IARDX );//OK
						str = MQ_EncInteger( RDy, str, codec, IARDY );//OK
						str = MQ_RefinementEncImage( (ImageSym->Image), (TextRegion->ImageSymT->Image), RefDx[kkk], RefDy[kkk], codec, str, TpGDon, TextRegion->SbrTemplate, TextRegion->RefATX1,  TextRegion->RefATY1,  TextRegion->RefATX2,  TextRegion->RefATY2 );
						TextImageCount++;
						Jbig2_ImageMarg( ImageTxt, (TextRegion->ImageSymT->Image), TextRegion->SbCombOp, Cur_T, Cur_S, TextRegion->RefCorner, 0, NULL);
						Cur_S += ((TextRegion->ImageSymT->Image->tbx1-TextRegion->ImageSymT->Image->tbx0-1)+ TextRegion->SbDsOffset );
					}
					else{
						ImageSym = Jb2_ImageChainSearch( ImageSym, RefID);
						Jbig2_ImageMarg(ImageTxt, ImageSym->Image, TextRegion->SbCombOp, Cur_T, Cur_S, TextRegion->RefCorner, 0, NULL );
						Cur_S += ((ImageSym->Image->tbx1-ImageSym->Image->tbx0-1)+ TextRegion->SbDsOffset);
					}
				}
				str = MQ_EncInteger( OOB, str, codec, IADS );
			}
			else{
				for(kkk=0 ; kkk<TextRegion->SbNumInstances ; kkk++){
					//Instance S
					if(FirstS){
						deltaS=Lx[kkk];
						str  = MQ_EncInteger( deltaS, str, codec, IAFS );//OK
						Cur_S = deltaS;	
						FirstS=0;
					}
					else{
						deltaS = Lx[kkk] - Cur_S;
						str = MQ_EncInteger( deltaS, str, codec, IADS );
						Cur_S = Lx[kkk];/**/
					}
					//Instance T
					Cur_T = Ly[kkk];
					deltaT = Cur_T - StripT;
					str = MQ_EncInteger( deltaT, str, codec, IAIT );

					//SymbolID
					str = MQ_EncIntegerIAID( ID[kkk], str, codec, SbSymbolCodeLength, IAID );
					nInstances++;

					ImageSym = Jb2_ImageChainSearch( ImageSym, ID[kkk] );
					Jbig2_ImageMarg(ImageTxt, ImageSym->Image, TextRegion->SbCombOp, Ly[kkk], Lx[kkk], TextRegion->RefCorner, 0, NULL);
					Cur_S += ((ImageSym->Image->tbx1-ImageSym->Image->tbx0-1)+ TextRegion->SbDsOffset);
				}
				str = MQ_EncInteger( OOB, str, codec, IADS );
			}
		} while(nInstances<TextRegion->SbNumInstances);
		str = MQ_flush(codec, str);
	}
	Jbig2_ImageMarg(ImagePage->Image, ImageTxt, JBIG2_XOR, TextRegion->RegionSegmentYlocation, TextRegion->RegionSegmentXlocation, JBIG2_TOP_LEFT, 0, NULL );//Original - ImageTxt->Image for GenericSegment
	if(TextRegion->ColourExtFlag){
		TempAddr = str->cur_p;
		str = T45_Enc( str, TextRegion->Col, TextRegion->numCmpts, TextRegion->CmptsL, TextRegion->num_Val);
		TempAddr = str->cur_p - TempAddr + 4;
		str = Stream4ByteWrite( str, TempAddr, str->buf_length, BIG_ENDIAN );
	}
#if JBIG2_DEBUG05
	char	fname[256];
//	strcpy(fname, "ImageTxt_Enc000");	
//	fname[14]=0x30+(TxtCounter%10);
//	fname[13]=0x30+((TxtCounter/10)%10);
//	fname[12]=0x30+((TxtCounter/100)%10);
//	strcat(fname, ".bmp");
//	Jb2_Debug_Print( fname, ImageTxt, 0, TextRegion->ColourExtFlag );

	strcpy(fname, "ImageResiEnc000");	
	fname[14]=0x30+(TxtCounter%10);
	fname[13]=0x30+((TxtCounter/10)%10);
	fname[12]=0x30+((TxtCounter/100)%10);
	strcat(fname, ".bmp");
	Jb2_Debug_Print( fname, ImagePage, 0, TextRegion->ColourExtFlag );
	TxtCounter++;
#endif
	delete	[] ImageTxt->Pdata;
	delete	ImageTxt;

	return	str;
}



struct StreamChain_s *PageInformationSegmentEnc( struct	PageInformationSegment_s *PageInfo, struct StreamChain_s *str )
{
	ubyte2	PageStripingInformation;
	uchar	PageSegmentFlags;

	str = Stream4ByteWrite( str, PageInfo->PageBitmapWidth, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, PageInfo->PageBitmapHeight, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, PageInfo->PageXResolution, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, PageInfo->PageYResolution, str->buf_length, BIG_ENDIAN);
	
	PageSegmentFlags = PageInfo->PageEventuallyLossless&1;
	PageSegmentFlags += ((PageInfo->PageMightContainRefinements&1)<<1);
	PageSegmentFlags += ((PageInfo->PageDefaultPixelValue&1)<<2); 
	PageSegmentFlags += ((PageInfo->PageDefaultCombinationOperator&3)<<3);
	PageSegmentFlags += ((PageInfo->PageRequiersAuxllaryBuffers&1)<<5);
	PageSegmentFlags += ((PageInfo->PageCombinationOperatorOverRidden&1)<<6);
	PageSegmentFlags += ((PageInfo->ColorExtFlag&1)<<7);
	str = Stream1ByteWrite( str, PageSegmentFlags, str->buf_length);

	PageStripingInformation = ((PageInfo->PageStriped&1)<<15);
	PageStripingInformation += (PageInfo->MaximumStripeSize&0x7fff);
	str = Stream2ByteWrite( str, PageStripingInformation, str->buf_length, BIG_ENDIAN);

	return	str;
}

struct StreamChain_s *EndOfPageSegmentEnc( struct Jbig2Parameter_s *Jb2Param, struct StreamChain_s *str )
{
	return	str;
}

//7.4.6 
struct StreamChain_s *ImmediateLosslessGenericRegionSegmentEnc( struct GenericRegionSegment_s *GenRegion, struct StreamChain_s *str, struct ImageChain_s *ImagePage, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec )
{
	byte4	j, width, height, x0, y0, x1, y1;
	byte4	Dcol1step, Gcol1step;
	uchar	GenericRegionSegmentFlags;
	struct	Image_s *ImageGen=NULL;
	uchar	*D_TS, *d_TS;
	char	RTC2_on=0;

	if( !GenRegion->RegionSegmentBitmapWidth )
		GenRegion->RegionSegmentBitmapWidth = ImagePage->Image->tbx1 - ImagePage->Image->tbx0;
	if( !GenRegion->RegionSegmentBitmapHeight )
		GenRegion->RegionSegmentBitmapHeight= ImagePage->Image->tby1 - ImagePage->Image->tby0;

	width = GenRegion->RegionSegmentBitmapWidth;
	height= GenRegion->RegionSegmentBitmapHeight;
	x0 = GenRegion->RegionSegmentXlocation;
	x1 = x0 + width;
	y0 = GenRegion->RegionSegmentYlocation;
	y1 = y0 + height;
	ImageGen = ImageCreate(ImageGen, width, height, 0, width, 0, height, CHAR);

	Gcol1step = ImageGen->col1step;
	Dcol1step = ImagePage->Image->col1step;
	D_TS = (uchar *)ImagePage->Image->data;
	d_TS = (uchar *)ImageGen->data;
	for( j=0, D_TS = &D_TS[Dcol1step*y0] ; j<height ; j++, D_TS=&D_TS[Dcol1step], d_TS=&d_TS[Gcol1step] ){
		memcpy( d_TS, &D_TS[x0], sizeof(uchar)*width );
	}
	//RegionSegmentInformationField
	str = Stream4ByteWrite( str, GenRegion->RegionSegmentBitmapWidth, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, GenRegion->RegionSegmentBitmapHeight, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, GenRegion->RegionSegmentXlocation, str->buf_length, BIG_ENDIAN);
	str = Stream4ByteWrite( str, GenRegion->RegionSegmentYlocation, str->buf_length, BIG_ENDIAN);
	str = Stream1ByteWrite( str, GenRegion->ExternalCombinationOperator, str->buf_length );

	//GenericRegionSegmentFlags
	GenericRegionSegmentFlags = (GenRegion->MMR&1);
	GenericRegionSegmentFlags += ((GenRegion->Template&3)<<1);
	GenericRegionSegmentFlags += ((GenRegion->ExtTemplate&1)<<4);
	GenericRegionSegmentFlags += ((GenRegion->TpGDon&1)<<3);
	str = Stream1ByteWrite(str, GenericRegionSegmentFlags, str->buf_length);

	if(!GenRegion->MMR){
		if(!GenRegion->ExtTemplate){
			if(!GenRegion->Template){
				str = Stream1ByteWrite(str, GenRegion->ATX1, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATY1, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATX2, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATY2, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATX3, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATY3, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATX4, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATY4, str->buf_length);
			}
			else{
				str = Stream1ByteWrite(str, GenRegion->ATX1, str->buf_length);
				str = Stream1ByteWrite(str, GenRegion->ATY1, str->buf_length);
			}
		}
		else if( GenRegion->ExtTemplate && GenRegion->Template==0 ){
			str = Stream1ByteWrite(str, GenRegion->ATX1, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY1, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX2, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY2, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX3, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY3, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX4, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY4, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX5, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY5, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX6, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY6, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX7, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY7, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX8, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY8, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX9, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY9, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX10, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY10, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX11, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY11, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATX12, str->buf_length);
			str = Stream1ByteWrite(str, GenRegion->ATY12, str->buf_length);
		}
	}

#if JBIG2_DEBUG01
	char	fname[256];
	strcpy(fname, "ImagePageResi2");	
	strcat(fname, ".bmp");
	Jb2_Debug_Print( fname, ImagePage, 0, 0 );
#endif

	//GenericRegionSegmentFlags
	if(GenRegion->MMR)
		str = T4T6Encmain(str, ImageGen, 0, T6, RTC2_on);
	else{
		InitMQ_Codec( codec, str, codec->numCX, ENC, str->buf_length, JBIG2 );
		str = MQ_EncImage( ImageGen, str, codec, GenRegion->TpGDon, GenRegion->Template, GenRegion->ExtTemplate, GenRegion->ATX1, GenRegion->ATY1, GenRegion->ATX2, GenRegion->ATY2, GenRegion->ATX3, GenRegion->ATY3, GenRegion->ATX4, GenRegion->ATY4, GenRegion->ATX5, GenRegion->ATY5, GenRegion->ATX6, GenRegion->ATY6, GenRegion->ATX7, GenRegion->ATY7, GenRegion->ATX8, GenRegion->ATY8, GenRegion->ATX9, GenRegion->ATY9, GenRegion->ATX10, GenRegion->ATY10, GenRegion->ATX11, GenRegion->ATY11, GenRegion->ATX12, GenRegion->ATY12, 0 );
		str = MQ_flush(codec, str);
	}

	delete	[] ImageGen->Pdata;
	delete	ImageGen;

	return	str;
}
