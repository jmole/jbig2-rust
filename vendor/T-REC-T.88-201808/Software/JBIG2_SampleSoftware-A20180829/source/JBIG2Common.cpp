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
#include	"Jb2_Debug.h"

struct Jb2_ImageChain_s *Jb2_ImageChainSearch( struct Jb2_ImageChain_s *ImageC, byte4 ID)
{
	if( ImageC->ID == ID )
		return	ImageC;

	ImageC = Jb2_ImageChainParentSearch( ImageC );
	while( ImageC->ID != ID ){
		ImageC=ImageC->child;
	}
	return	ImageC;
}

struct Jb2_ImageChain_s *Jb2_ImageChainParentSearch( struct Jb2_ImageChain_s *ImageC )
{
	while(ImageC->parent !=NULL ){
		ImageC=ImageC->parent;
	}
	return	ImageC;
}

struct Jb2_ImageChain_s *Jb2_ImageChainChildSearch( struct Jb2_ImageChain_s *ImageC )
{
	while(ImageC->child !=NULL ){
		ImageC=ImageC->child;
	}
	return	ImageC;
}

struct Jb2_ImageChain_s *Jb2_ImageChainCreate( struct Jb2_ImageChain_s *Parent)
{
	struct	Jb2_ImageChain_s *ImageC;
	if(Parent!=NULL){
		Parent = Jb2_ImageChainChildSearch(Parent);
		if( NULL == (ImageC = new struct Jb2_ImageChain_s) ){
			printf("[Jb2_ImageChainCreate] Jb2_ImageChain_s create error\n");
			return	NULL;
		}
		ImageC->Image=NULL;
		ImageC->parent = Parent;
		ImageC->child=NULL;
		ImageC->numChains = Parent->numChains+1;
		ImageC->ID =0;
		ImageC->FileID=0;
		Parent->child = ImageC;
	}
	else{
		if( NULL == (ImageC = new struct Jb2_ImageChain_s) ){
			printf("[Jb2_ImageChainCreate] Jb2_ImageChain_screate error\n");
			return	NULL;
		}
		ImageC->Image=NULL;
		ImageC->parent=NULL;
		ImageC->child=NULL;
		ImageC->numChains=0;
		ImageC->ID=0;
		ImageC->FileID=0;
	}
	return ImageC;
}


struct Jbig2Parameter_s *Jb2ParamInit( char *fname_ini, struct ImageChain_s *ImagePage, char enc_dec )
{
	FILE	*fp;
	byte4	NumberOfSegments=0;	//PageInfoSegment + PageEndSegment
	byte4	SegNo;
	byte4	data, data2;
	byte4	HeightClassD, WidthClassD, kkk;
	byte4	SdNumExSyms=0;
	uchar	sym=0, txt=0, pat=0, half=0, gen=0;
	char	Flag=0, Flag2=0;
	struct	Jbig2Parameter_s *Jb2Param;
	struct	TextRegionSegment_s *TextRegion=NULL;
	struct	Image_s *rImage=NULL;
	struct	Jb2_ImageChain_s *ImageSym=NULL;
	struct	ImageChain_s *ImagePat=NULL;

#if JBIG2_DEBUG00
	printf(" Jb2ParamInit IN \n");
#endif
	Jb2Param = new struct Jbig2Parameter_s;

	Jb2Param->NumberOfSegments = 0;
	Jb2Param->PatternDic= NULL;
	Jb2Param->HalfRegion= NULL;
	Jb2Param->TextRegion= NULL;
	Jb2Param->SymbolDic = NULL;
	Jb2Param->GenRegion = NULL;
	Jb2Param->ImagePat=NULL;
	Jb2Param->ImageHaf=NULL;
	Jb2Param->ImageSym=NULL;
	Jb2Param->ImageTxt=NULL;
	Jb2Param->ImageGen=NULL;

	Jb2Param->PageInfo= new struct  PageInformationSegment_s;
	Jb2Param->NumberOfSegments++;
	Jb2Param->PageInfo->SegNo=0;
	Jb2Param->PageInfo->PageAssociationSize=0;
	Jb2Param->PageInfo->SegmentType=PAGE_INFORMATION;
	Jb2Param->PageInfo->retainMe=0;
	Jb2Param->PageInfo->numReferSeg=0;
	Jb2Param->PageInfo->pRetainBits=NULL;
	Jb2Param->PageInfo->pReferSegNo=NULL;
	Jb2Param->PageInfo->PageAssociate=1;
	Jb2Param->PageInfo->SegmentDataLength=0;
	Jb2Param->PageInfo->PageBitmapWidth=0;
	Jb2Param->PageInfo->PageBitmapHeight=0;
	Jb2Param->PageInfo->PageXResolution=0;
	Jb2Param->PageInfo->PageYResolution=0;
	Jb2Param->PageInfo->PageEventuallyLossless=1;
	Jb2Param->PageInfo->PageMightContainRefinements=0;
	Jb2Param->PageInfo->PageDefaultPixelValue=0;
	Jb2Param->PageInfo->PageDefaultCombinationOperator=JBIG2_OR;
	Jb2Param->PageInfo->PageRequiersAuxllaryBuffers=0;
	Jb2Param->PageInfo->PageCombinationOperatorOverRidden=0;
	Jb2Param->PageInfo->PageStriped=0;
	Jb2Param->PageInfo->ColorExtFlag=0;
	Jb2Param->PageInfo->MaximumStripeSize=0;
	if(enc_dec==ENC){
		Jb2Param->PageInfo->PageBitmapWidth=(ubyte4)(ImagePage->Image->tbx1-ImagePage->Image->tbx0);
		Jb2Param->PageInfo->PageBitmapHeight=(ubyte4)(ImagePage->Image->tby1-ImagePage->Image->tby0);
	}

	if(enc_dec==ENC){
		if( fname_ini!=NULL ){
			if( NULL != (fp=fopen( fname_ini,"r")) ){
				char	fname[256], mode[256];
				while( EOF!=fscanf(fp,"%s",mode) ){
					if( !strcmp(mode, "-sym") || (!strcmp(mode, "-Sym")) ){
						fscanf(fp,"%s", mode);
						if( !strcmp(mode, "-Seg") ){
							fscanf(fp,"%d", &SegNo);
							Jb2Param->SymbolDic = SymbolDicCreate( Jb2Param->SymbolDic );
							SymbolDic_Initiate( Jb2Param->SymbolDic );
							Jb2Param->NumberOfSegments++;
							Jb2Param->SymbolDic->ImageSym = NULL;
							Jb2Param->SymbolDic->SegNo = SegNo;
						}
						else if(!strcmp(mode, "-file") ){
							fscanf(fp,"%s\n",mode);
							if( !strcmp(mode, "-EndOfHeightClass") ){
								Jb2Param->SymbolDic->numWidth[HeightClassD]=WidthClassD;
								HeightClassD++;
							}
							else if( !strcmp(mode, "-numClass") ){
								fscanf(fp,"%s", fname);
								fscanf(fp,"%d\n",&data);
								Jb2Param->SymbolDic->Height = new byte4 [data];
								Jb2Param->SymbolDic->numWidth = new byte4 [data];
								memset(Jb2Param->SymbolDic->Height, 0, sizeof(byte4)*data);
								memset(Jb2Param->SymbolDic->numWidth, 0, sizeof(byte4)*data);
								Jb2Param->SymbolDic->numHeight = data;
								Jb2Param->SymbolDic->Width = new byte4* [Jb2Param->SymbolDic->numHeight];
								fscanf(fp,"%s", fname);
								fscanf(fp, "%d\n", &data);
								for(kkk=0 ; kkk<Jb2Param->SymbolDic->numHeight; kkk++){
									Jb2Param->SymbolDic->Width[kkk] = new byte4 [data];
									memset(Jb2Param->SymbolDic->Width[kkk], 0, sizeof(byte4)*data);
								}
								HeightClassD=0;
								WidthClassD =0;
							}
							else if( !strcmp(mode, "-numSymbol") ){
								fscanf(fp,"%d\n",&data);
								Jb2Param->SymbolDic->SdNumNewSyms=data;
								SdNumExSyms+=data;
								Jb2Param->SymbolDic->SdNumExSyms=SdNumExSyms;
							}
							else if( !strcmp(mode, "-Height") ){
								fscanf(fp,"%d\n",&data);
								Jb2Param->SymbolDic->Height[HeightClassD]=data;
								WidthClassD=0;
							}
							else if( !strcmp(mode, "-Width") ){
								fscanf(fp,"%d",&data);
								Jb2Param->SymbolDic->Width[HeightClassD][WidthClassD]=data;
								WidthClassD++;
								/**/
								fscanf(fp, "%s", mode);
								if(!strcmp(mode,"-Simple") ){
									Jb2Param->SymbolDic->RefinementAggregate =0;
									Jb2Param->SymbolDic->RefAggnInst = 0;
									fscanf(fp,"%d",&data);
									fscanf(fp,"%s\n", fname);
									if( (rImage = (struct Image_s *)LoadBmp(fname))== (struct Image_s*)FALES ){
										printf("[Jb2ParamInit] Symbol Data Load Error\n");
										return	(struct	Jbig2Parameter_s *)FALES;
									}
									ImageSym = Jb2_ImageChainCreate(ImageSym);
									ImageSym->Image = (struct Image_s *)ImageBit1ToChar(rImage);
									ImageSym->ID = data;
									ImageSym->RefID = -1;

								}
								else if(!strcmp(mode, "-Ref") ){
									Jb2Param->SymbolDic->RefinementAggregate =1;
									Jb2Param->SymbolDic->RefAggnInst = Jb2Param->SymbolDic->SdNumNewSyms;
									fscanf(fp,"%d",&data);
									fscanf(fp,"%s\n", fname);
									if( (rImage=(struct Image_s *)LoadBmp(fname))==NULL ){
										printf("[Jb2ParamInit] Referance Symbol Data Load Error\n");
										return	NULL;
									}
									ImageSym = Jb2_ImageChainCreate( ImageSym );
									ImageSym->Image = (struct Image_s *)ImageBit1ToChar(rImage);
									ImageSym->ID = data;
									fscanf(fp,"%d",&data);
									ImageSym->RefID = data;
									fscanf(fp,"%d",&data);
									ImageSym->RDx = data;
									fscanf(fp,"%d",&data);
									ImageSym->RDy = data;
								}
							}
						}
						else if(!strcmp(mode, "-Param") ){
							fscanf(fp,"%s", mode);
							fscanf(fp,"%d\n", &data);
							if(	!strcmp(mode, "-Huff") ){
								Jb2Param->SymbolDic->Huff = data;
							}
							else if( !strcmp(mode, "-Huff_DH") ){
								Jb2Param->SymbolDic->Huff_DH_Selection = data;
							}
							else if( !strcmp(mode, "-Huff_DW") ){
								Jb2Param->SymbolDic->Huff_DW_Selection = data;
							}
							else if( !strcmp(mode, "-Huff_BM") ){
								Jb2Param->SymbolDic->Huff_BmSize_Selection = data;
							}
							else if( !strcmp(mode, "-Huff_BMSize") ){
								Jb2Param->SymbolDic->BmSize = data;
							}
							else if( !strcmp(mode, "-Huff_AGG") ){
								Jb2Param->SymbolDic->Huff_Agginst_Selection = data;
							}
							else if( !strcmp(mode, "-Template") ){
								Jb2Param->SymbolDic->Template = data;
							}
							else if( !strcmp(mode, "-ATX1") ){
								Jb2Param->SymbolDic->ATX1 = data;
							}
							else if( !strcmp(mode, "-ATY1") ){
								Jb2Param->SymbolDic->ATY1 = data;
							}
							else if( !strcmp(mode, "-ATX2") ){
								Jb2Param->SymbolDic->ATX2 = data;
							}
							else if( !strcmp(mode, "-ATY2") ){
								Jb2Param->SymbolDic->ATY2 = data;
							}
							else if( !strcmp(mode, "-ATX3") ){
								Jb2Param->SymbolDic->ATX3 = data;
							}
							else if( !strcmp(mode, "-ATY3") ){
								Jb2Param->SymbolDic->ATY3 = data;
							}
							else if( !strcmp(mode, "-ATX4") ){
								Jb2Param->SymbolDic->ATX4 = data;
							}
							else if( !strcmp(mode, "-ATY4") ){
								Jb2Param->SymbolDic->ATY4 = data;
							}
							else if( !strcmp(mode, "-RefTemplate") ){
								Jb2Param->SymbolDic->RefTemplate = data;
							}
							else if( !strcmp(mode, "-rATX1") ){
								Jb2Param->SymbolDic->RefATX1 = data;
							}
							else if( !strcmp(mode, "-rATY1") ){
								Jb2Param->SymbolDic->RefATY1 = data;
							}
							else if( !strcmp(mode, "-rATX2") ){
								Jb2Param->SymbolDic->RefATX2 = data;
							}
							else if( !strcmp(mode, "-rATY2") ){
								Jb2Param->SymbolDic->RefATY2 = data;
							}
						}
					}
					else if( !strcmp(mode, "-txt") || (!strcmp(mode, "-Txt")) ){
						fscanf(fp,"%s", mode);
						if( !strcmp(mode, "-Seg") ){
							fscanf(fp,"%d\n", &SegNo);
							TextRegion = TextRegionCreate( TextRegion );
							TextRegion_Initiate(TextRegion);
							TextRegion->SegNo = SegNo;
							TextRegion->ImageSymT=NULL;
							Jb2Param->NumberOfSegments++;
							Jb2Param->TextRegion = TextRegion;
						}
						else if(!strcmp(mode, "-Param") ){
							fscanf(fp,"%s", mode);
							if(	!strcmp(mode, "-Huff") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->Huff = data;
							}
							else if( !strcmp(mode, "-numInst") ){
								fscanf(fp,"%d", &data);
								TextRegion->SbNumInstances = data;
								TextRegion->ID = new byte4 [data];
								TextRegion->Lx = new byte4 [data];
								TextRegion->Ly = new byte4 [data];
								TextRegion->RefDx = new byte4 [data];
								TextRegion->RefDy = new byte4 [data];
								for(kkk=0; kkk<TextRegion->SbNumInstances ; kkk++){
									fscanf(fp,"%s", mode);
									if( !strcmp(mode, "-ID") ){
										fscanf(fp,"%d", &data);
										TextRegion->ID[kkk] = data;
										fscanf(fp,"%d", &data);
										TextRegion->Ly[kkk] = data;
										fscanf(fp,"%d", &data);
										TextRegion->Lx[kkk] = data;
										TextRegion->RefDx[kkk] = 0;
										TextRegion->RefDy[kkk] = 0;
									}
									else if( !strcmp(mode, "-RefID") ){
										TextRegion->Refine=1;
										fscanf(fp,"%d", &data);
										TextRegion->ID[kkk] = (0x80000000+data);
										fscanf(fp, "%s", fname );
										if( (rImage = (struct Image_s *)LoadBmp(fname))==NULL ){
											printf("[Jb2ParamInit] TextRegion RefData Load Error\n");
											return	NULL;
										}
										TextRegion->ImageSymT = Jb2_ImageChainCreate( TextRegion->ImageSymT );
										TextRegion->ImageSymT->Image = (struct Image_s *)ImageBit1ToChar(rImage);
										fscanf(fp,"%d", &data);
										TextRegion->Ly[kkk] = data;
										fscanf(fp,"%d", &data);
										TextRegion->Lx[kkk] = data;
										fscanf(fp,"%d", &data);
										TextRegion->RefDx[kkk] = data;
										fscanf(fp,"%d", &data);
										TextRegion->RefDy[kkk] = data;
									}
								}
							}
							else if( !strcmp(mode, "-numPich") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->SbDsOffset = data;
							}
							else if( !strcmp(mode, "-W") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RegionSegmentBitmapWidth = data;
							}
							else if( !strcmp(mode, "-H") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RegionSegmentBitmapHeight = data;
							}
							else if( !strcmp(mode, "-ColorExt") ){
								Jb2Param->PageInfo->ColorExtFlag=1;
								TextRegion->ColourExtFlag = 1;
								fscanf(fp,"%d\n", &data2);
								TextRegion->num_Val = data2;
								TextRegion->CmptsL = 1;
								TextRegion->numCmpts = 3;
								TextRegion->Col = new byte4 [data2*3];
								for(kkk=0; kkk<data2 ; kkk++ ){
									fscanf(fp,"%d\n", &data);
									TextRegion->Col[kkk*3] = data;
									fscanf(fp,"%d\n", &data);
									TextRegion->Col[kkk*3+1] = data;
									fscanf(fp,"%d\n", &data);
									TextRegion->Col[kkk*3+2] = data;
								}					
							}
							else if( !strcmp(mode, "-RefCorner") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RefCorner = data;
							}
							else if( !strcmp(mode, "-Huff_Ds") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffDsSelection = data;
							}
							else if( !strcmp(mode, "-Huff_Dt") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffDtSelection = data;
							}
							else if( !strcmp(mode, "-Huff_Fs") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffFsSelection = data;
							}
							else if( !strcmp(mode, "-Huff_RdH") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffRdHSelection = data;
							}
							else if( !strcmp(mode, "-Huff_RdW") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffRdWSelection = data;
							}
							else if( !strcmp(mode, "-Huff_RdY") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffRdYSelection = data;
							}
							else if( !strcmp(mode, "-Huff_RdX") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->HuffRdXSelection = data;
							}
							else if( !strcmp(mode, "-Ylocation") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RegionSegmentYlocation = data;
							}
							else if( !strcmp(mode, "-Xlocation") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RegionSegmentXlocation = data;
							}
							else if( !strcmp(mode, "-RefTemplate") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->SbrTemplate = data;
							}
							else if( !strcmp(mode, "-rATX1") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RefATX1 = data;
							}
							else if( !strcmp(mode, "-rATY1") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RefATY1 = data;
							}
							else if( !strcmp(mode, "-rATX2") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RefATX2 = data;
							}
							else if( !strcmp(mode, "-rATY2") ){
								fscanf(fp,"%d\n", &data);
								TextRegion->RefATY2 = data;
							}
						}
					}
					else if( !strcmp(mode, "-Gen") || (!strcmp(mode, "-gen")) ){
						fscanf(fp,"%s", mode);
						if( !strcmp(mode, "-Seg") ){
							fscanf(fp,"%d\n", &SegNo);				
							Jb2Param->GenRegion = GenRegionCreate(Jb2Param->GenRegion);
							Jb2Param->GenRegion->SegNo = SegNo;
							Jb2Param->NumberOfSegments++;
							GenRegion_Initiate(Jb2Param->GenRegion);
						}
						else if(!strcmp(mode, "-Param") ){
							fscanf(fp,"%s", mode);
							if(	!strcmp(mode, "-Huff") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->MMR = data;
							}
							else if( !strcmp(mode, "-Ylocation") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->RegionSegmentYlocation = data;
							}
							else if( !strcmp(mode, "-Xlocation") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->RegionSegmentXlocation = data;
							}
							else if( !strcmp(mode, "-W") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->RegionSegmentBitmapWidth = data;
							}
							else if( !strcmp(mode, "-H") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->RegionSegmentBitmapHeight = data;
							}
							else if( !strcmp(mode, "-Template") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->Template = data;
							}
							else if( !strcmp(mode, "-ExtTemplate") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ExtTemplate = data;
							}
							else if( !strcmp(mode, "-TpGDon") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->TpGDon = data;
							}
							else if( !strcmp(mode, "-ATX1") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX1 = data;
							}
							else if( !strcmp(mode, "-ATY1") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY1 = data;
							}
							else if( !strcmp(mode, "-ATX2") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX2 = data;
							}
							else if( !strcmp(mode, "-ATY2") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY2 = data;
							}
							else if( !strcmp(mode, "-ATX3") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX3 = data;
							}
							else if( !strcmp(mode, "-ATY3") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY3 = data;
							}
							else if( !strcmp(mode, "-ATX4") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX4 = data;
							}
							else if( !strcmp(mode, "-ATY4") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY4 = data;
							}
							else if( !strcmp(mode, "-ATX5") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX5 = data;
							}
							else if( !strcmp(mode, "-ATY5") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY5 = data;
							}
							else if( !strcmp(mode, "-ATX6") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX6 = data;
							}
							else if( !strcmp(mode, "-ATY6") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY6 = data;
							}
							else if( !strcmp(mode, "-ATX7") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX7 = data;
							}
							else if( !strcmp(mode, "-ATY7") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY7 = data;
							}
							else if( !strcmp(mode, "-ATX8") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX8 = data;
							}
							else if( !strcmp(mode, "-ATY8") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY8 = data;
							}
							else if( !strcmp(mode, "-ATX9") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX9 = data;
							}
							else if( !strcmp(mode, "-ATY9") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY9 = data;
							}
							else if( !strcmp(mode, "-ATX10") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX10 = data;
							}
							else if( !strcmp(mode, "-ATY10") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY10 = data;
							}
							else if( !strcmp(mode, "-ATX11") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX11 = data;
							}
							else if( !strcmp(mode, "-ATY11") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY11 = data;
							}
							else if( !strcmp(mode, "-ATX12") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATX12 = data;
							}
							else if( !strcmp(mode, "-ATY12") ){
								fscanf(fp,"%d\n", &data);
								Jb2Param->GenRegion->ATY12 = data;
							}
						}
					}
				}
				fclose(fp);
			}
		}
		if(Jb2Param->GenRegion == NULL){
			Jb2Param->GenRegion = GenRegionCreate( Jb2Param->GenRegion);
			Jb2Param->NumberOfSegments++;
			GenRegion_Initiate( Jb2Param->GenRegion);
			Jb2Param->GenRegion->RegionSegmentBitmapWidth=(ubyte4)(ImagePage->Image->tbx1-ImagePage->Image->tbx0);
			Jb2Param->GenRegion->RegionSegmentBitmapHeight=(ubyte4)(ImagePage->Image->tby1-ImagePage->Image->tby0);
		}
		Jb2Param->EndPage = new struct EndOfPageSegment_s;
		Jb2Param->NumberOfSegments++;
		Jb2Param->EndPage->SegNo=0;
		Jb2Param->EndPage->PageAssociationSize=0;
		Jb2Param->EndPage->SegmentType=END_OF_PAGE;
		Jb2Param->EndPage->retainMe=0;
		Jb2Param->EndPage->numReferSeg=0;
		Jb2Param->EndPage->pRetainBits=NULL;
		Jb2Param->EndPage->pReferSegNo=NULL;
		Jb2Param->EndPage->PageAssociate=1;
		Jb2Param->EndPage->SegmentDataLength=0;
	}
	else{//DEC
		Jb2Param->GenRegion = GenRegionCreate( Jb2Param->GenRegion);
		Jb2Param->NumberOfSegments++;
		GenRegion_Initiate( Jb2Param->GenRegion);
		Jb2Param->EndPage = new struct EndOfPageSegment_s;
		Jb2Param->NumberOfSegments++;
		Jb2Param->EndPage->SegNo=0;
		Jb2Param->EndPage->PageAssociationSize=0;
		Jb2Param->EndPage->SegmentType=END_OF_PAGE;
		Jb2Param->EndPage->retainMe=0;
		Jb2Param->EndPage->numReferSeg=0;
		Jb2Param->EndPage->pRetainBits=NULL;
		Jb2Param->EndPage->pReferSegNo=NULL;
		Jb2Param->EndPage->PageAssociate=0;
		Jb2Param->EndPage->SegmentDataLength=0;
	}

	Jb2Param->ImageSym = ImageSym;
#if JBIG2_DEBUG00
	printf("[Jb2ParamInit] Jb2ParamInit OUT \n");
#endif
	return	 Jb2Param;
}

struct TextRegionSegment_s *TextRegionCreate( struct TextRegionSegment_s *TextRegion )
{
	struct TextRegionSegment_s *TextRegionC;

	if(TextRegion==NULL){
		TextRegion = new struct TextRegionSegment_s;
		TextRegion->Child=NULL;
		TextRegion->Parent=NULL;
	}
	else{
		TextRegionC = new struct TextRegionSegment_s;
		TextRegion->Child   = TextRegionC;
		TextRegionC->Parent = TextRegion;
		TextRegionC->Child  = NULL;
		TextRegion = TextRegionC;
	}
	return	TextRegion;
}

struct SymbolDictionarySegment_s *SymbolDicCreate( struct SymbolDictionarySegment_s *SymbolDic )
{
	struct SymbolDictionarySegment_s *SymbolDicC;

	if(SymbolDic==NULL){
		SymbolDic = new struct SymbolDictionarySegment_s;
		SymbolDic->Child=NULL;
		SymbolDic->Parent=NULL;
	}
	else{
//		SymbolDic = SymbolDicSearch(SymbolDic, -1);
		SymbolDicC = new struct SymbolDictionarySegment_s;
		SymbolDic->Child   = SymbolDicC;
		SymbolDicC->Parent = SymbolDic;
		SymbolDicC->Child  = NULL;
		SymbolDic = SymbolDicC;
	}
	return	SymbolDic;
}

struct GenericRegionSegment_s *GenRegionCreate( struct GenericRegionSegment_s *GenRegion )
{
	struct GenericRegionSegment_s *GenRegionC;

//	GenRegion = new struct GenericRegionSegment_s;
	if(GenRegion==NULL){
		GenRegion = new struct GenericRegionSegment_s;
		GenRegion->Child=NULL;
		GenRegion->Parent=NULL;
	}
	else{
		GenRegion = GenRegionSearch(GenRegion, -1);
		GenRegionC = new struct GenericRegionSegment_s;
		GenRegion->Child   = GenRegionC;
		GenRegionC->Parent = GenRegion;
		GenRegionC->Child  = NULL;
		GenRegion = GenRegionC;
	}
	return	GenRegion;
}

struct TextRegionSegment_s *TextRegionSearch(struct TextRegionSegment_s *TextRegion, char SegNo)
{
	while( TextRegion->Parent != NULL )
		TextRegion = TextRegion->Parent;

	if(SegNo==0)
		return	TextRegion;
	else if(SegNo==-1){	//most Child Search
		while(TextRegion->Child!=NULL)
			TextRegion = TextRegion->Child;
		return	TextRegion;
	}
	else{
		for( ; SegNo>0 ; SegNo-- )
			TextRegion = TextRegion->Child;
		return	TextRegion;
	}
}

struct SymbolDictionarySegment_s *SymbolDicSearch( struct SymbolDictionarySegment_s *SymbolDic, char SegNo )
{
	while( SymbolDic->Parent != NULL )
		SymbolDic = SymbolDic->Parent;

	if(SegNo==0)
		return	SymbolDic;
	else if(SegNo==-1){		//most Child Search
		while(SymbolDic->Child!=NULL)
			SymbolDic = SymbolDic->Child;
		return	SymbolDic;
	}
	else{
		for(; SegNo>0; SegNo--)
			SymbolDic = SymbolDic->Child;
		return	SymbolDic;
	}
}

struct GenericRegionSegment_s *GenRegionSearch( struct GenericRegionSegment_s *GenRegion, char SegNo )
{
	while( GenRegion->Parent != NULL )
		GenRegion = GenRegion->Parent;

	if(SegNo==-1)//most Child Search
		while(GenRegion->Child!=NULL)
			GenRegion = GenRegion->Child;
	else
		for(; SegNo>0; SegNo--)
			GenRegion = GenRegion->Child;
	return	GenRegion;
}

void GenRegion_Initiate(struct GenericRegionSegment_s *GenRegion)
{
//	GenRegion->SegNo=0;
	GenRegion->PageAssociationSize=0;
	GenRegion->SegmentType=IMMEDIATE_LOSLESS_GENERIC_REGION;
	GenRegion->retainMe=0;
	GenRegion->numReferSeg=0;
	GenRegion->pRetainBits=NULL;
	GenRegion->pReferSegNo=NULL;
	GenRegion->PageAssociate=1;
	GenRegion->SegmentDataLength=0;
	GenRegion->RegionSegmentBitmapWidth=0;		//RegionSegmentInfoField Region7.4.1(P50)
	GenRegion->RegionSegmentBitmapHeight=0;		//RegionSegmentInfoField
	GenRegion->RegionSegmentXlocation=0;			//RegionSegmentInfoField
	GenRegion->RegionSegmentYlocation=0;			//RegionSegmentInfoField
	GenRegion->ExternalCombinationOperator=JBIG2_XOR;	//RegionSegmentInfoField
	GenRegion->MMR=0;
	GenRegion->Template=0;
	GenRegion->ExtTemplate=0;
	GenRegion->TpGDon=0;
	GenRegion->ATX1=3;
	GenRegion->ATY1=-1;
	GenRegion->ATX2=-3;
	GenRegion->ATY2=-1;
	GenRegion->ATX3=2;
	GenRegion->ATY3=-2;
	GenRegion->ATX4=-2;
	GenRegion->ATY4=-2;
	GenRegion->ATX5=0;
	GenRegion->ATY5=0;
	GenRegion->ATX6=0;
	GenRegion->ATY6=0;
	GenRegion->ATX7=0;
	GenRegion->ATY7=0;
	GenRegion->ATX8=0;
	GenRegion->ATY8=0;
	GenRegion->ATX9=0;
	GenRegion->ATY9=0;
	GenRegion->ATX10=0;
	GenRegion->ATY10=0;
	GenRegion->ATX11=0;
	GenRegion->ATY11=0;
	GenRegion->ATX12=0;
	GenRegion->ATY12=0;

}

void SymbolDic_Initiate(struct SymbolDictionarySegment_s *SymbolDic)
{
	SymbolDic->ATX1=3;
	SymbolDic->ATY1=-1;
	SymbolDic->ATX2=-3;
	SymbolDic->ATY2=-1;
	SymbolDic->ATX3=2;
	SymbolDic->ATY3=-2;
	SymbolDic->ATX4=-2;
	SymbolDic->ATY4=-2;
	SymbolDic->BitmapCodingContextRetained=0;
	SymbolDic->BitmapCodingContextUsed=0;
	SymbolDic->Huff=0;
	SymbolDic->BmSize=0;
	SymbolDic->Huff_Agginst_Selection=0;
	SymbolDic->Huff_DH_Selection=1;
	SymbolDic->Huff_DW_Selection=1;
	SymbolDic->Huff_BmSize_Selection=0;
	SymbolDic->numReferSeg=0;
	SymbolDic->PageAssociate=1;
	SymbolDic->PageAssociationSize=0;
	SymbolDic->Template=0;
	SymbolDic->pReferSegNo=NULL;
	SymbolDic->pRetainBits=NULL;
	SymbolDic->RefinementAggregate=0;
	SymbolDic->RefATX1=0;
	SymbolDic->RefATY1=0;
	SymbolDic->RefATX2=0;
	SymbolDic->RefATY2=0;
	SymbolDic->RefTemplate=0;
	SymbolDic->numSymbol_Page=0;
	SymbolDic->numSymbol_File=0;
	SymbolDic->SdNumExSyms=0;
	SymbolDic->SdNumNewSyms=0;
	SymbolDic->SegmentDataLength=0;
	SymbolDic->SegmentType=SYMBOL_DICTIONARY;
}

void TextRegion_Initiate(struct	TextRegionSegment_s *TextRegion)
{
	TextRegion->numReferSeg=0;
	TextRegion->PageAssociate=1;
	TextRegion->PageAssociationSize=0;
	TextRegion->pReferSegNo=NULL;
	TextRegion->pRetainBits=NULL;
	TextRegion->SegmentDataLength=0;
	TextRegion->SegmentType=IMMEDIATE_LOSLESS_TEXT_REGION;

	TextRegion->RegionSegmentBitmapWidth=0;	
	TextRegion->RegionSegmentBitmapHeight=0;
	TextRegion->RegionSegmentXlocation=0;
	TextRegion->RegionSegmentYlocation=0;
	TextRegion->ExternalCombinationOperator=JBIG2_OR;

	TextRegion->Huff=0;
	TextRegion->Refine=0;
	TextRegion->LogSbStrips=2;	//Strip=4
	TextRegion->RefCorner=0;
	TextRegion->Transposed=0;
	TextRegion->SbCombOp=JBIG2_OR;
	TextRegion->SbDefPixel=0;	
	TextRegion->SbDsOffset=3;
	TextRegion->SbrTemplate=0;
	TextRegion->HuffFsSelection=0;
	TextRegion->HuffDsSelection=0;
	TextRegion->HuffDtSelection=0;
	TextRegion->HuffRdWSelection=0;
	TextRegion->HuffRdHSelection=0;
	TextRegion->HuffRdXSelection=0;
	TextRegion->HuffRdYSelection=0;
	TextRegion->HuffRSizeSelection=0;
	TextRegion->RefATX1=0;				//SymbolDictionaryATFlags
	TextRegion->RefATY1=0;				//SymbolDictionaryATFlags
	TextRegion->RefATX2=0;				//SymbolDictionaryATFlags
	TextRegion->RefATY2=0;				//SymbolDictionaryATFlags
	TextRegion->SbNumInstances=0;
	TextRegion->ColourExtFlag=0;
	TextRegion->Col=NULL;
}

struct Jb2SegmentHeader_s *SegmentCreate( struct Jbig2Parameter_s *Jb2Param )
{
	struct Jb2SegmentHeader_s *Seg;
	struct	TextRegionSegment_s			*TextRegion;
	struct	SymbolDictionarySegment_s	*SymbolDic;
	struct	GenericRegionSegment_s		*GenRegion;
	byte4	txtNumber=0, symNumber=0;
	char	i, Flag;

	Seg = new struct Jb2SegmentHeader_s [Jb2Param->NumberOfSegments];
	TextRegion    = Jb2Param->TextRegion;
	SymbolDic     = Jb2Param->SymbolDic;
	GenRegion     = Jb2Param->GenRegion;

	Seg[0].SegNo=0;
	Seg[0].SegmentType =Jb2Param->PageInfo->SegmentType;
	Seg[0].PageAssociationSize = Jb2Param->PageInfo->PageAssociationSize;
	Seg[0].PageAssociate = Jb2Param->PageInfo->PageAssociate;
	Seg[0].retainMe = Jb2Param->PageInfo->retainMe;;
	Seg[0].numReferSeg = Jb2Param->PageInfo->numReferSeg;
	Seg[0].pRetainBits=Jb2Param->PageInfo->pRetainBits;
	Seg[0].pReferSegNo = Jb2Param->PageInfo->pReferSegNo;
	Seg[0].PageAssociate = Jb2Param->PageInfo->PageAssociate;
	Seg[0].SegmentDataLength=Jb2Param->PageInfo->SegmentDataLength;
	Seg[0].PageInfo = Jb2Param->PageInfo;
	Seg[0].SymbolDic=NULL;
	Seg[0].TextRegion=NULL;
	Seg[0].PatternDic=NULL;
	Seg[0].HalfRegion=NULL;
	Seg[0].GenericRegion=NULL;
	Seg[0].EndPage=NULL;

	if( SymbolDic==NULL ){
		Seg[1].SegNo=1;
		Seg[1].SegmentType = Jb2Param->GenRegion->SegmentType;
		Seg[1].PageAssociationSize = Jb2Param->GenRegion->PageAssociationSize;
		Seg[1].PageAssociate = Jb2Param->GenRegion->PageAssociate;
		Seg[1].retainMe = Jb2Param->GenRegion->retainMe;;
		Seg[1].numReferSeg = Jb2Param->GenRegion->numReferSeg;
		Seg[1].pRetainBits=Jb2Param->GenRegion->pRetainBits;
		Seg[1].pReferSegNo = Jb2Param->GenRegion->pReferSegNo;
		Seg[1].PageAssociate = Jb2Param->GenRegion->PageAssociate;
		Seg[1].SegmentDataLength=Jb2Param->GenRegion->SegmentDataLength;
		Seg[1].PageInfo = NULL;
		Seg[1].SymbolDic = NULL;
		Seg[1].TextRegion =NULL;
		Seg[1].PatternDic=NULL;
		Seg[1].HalfRegion=NULL;
		Seg[1].GenericRegion = Jb2Param->GenRegion;
		Seg[1].EndPage=NULL;

		Seg[2].SegNo=2;
		Seg[2].SegmentType = Jb2Param->EndPage->SegmentType;
		Seg[2].PageAssociationSize = Jb2Param->EndPage->PageAssociationSize;
		Seg[2].PageAssociate = Jb2Param->EndPage->PageAssociate;
		Seg[2].retainMe = Jb2Param->EndPage->retainMe;;
		Seg[2].numReferSeg = Jb2Param->EndPage->numReferSeg;
		Seg[2].pRetainBits=Jb2Param->EndPage->pRetainBits;
		Seg[2].pReferSegNo = Jb2Param->EndPage->pReferSegNo;
		Seg[2].PageAssociate = Jb2Param->EndPage->PageAssociate;
		Seg[2].SegmentDataLength=Jb2Param->EndPage->SegmentDataLength;
		Seg[2].PageInfo = NULL;
		Seg[2].SymbolDic = NULL;
		Seg[2].TextRegion =NULL;
		Seg[2].PatternDic=NULL;
		Seg[2].HalfRegion=NULL;
		Seg[2].GenericRegion =NULL;
		Seg[2].EndPage = Jb2Param->EndPage;
		return	Seg;
	}

	SymbolDic = SymbolDicSearch( SymbolDic, 0);
	i=1;
	Flag=1;
	while(SymbolDic != NULL && Flag ){
		Seg[i].SegNo=i;
		Seg[i].SegmentType = SymbolDic->SegmentType;
		Seg[i].PageAssociationSize = SymbolDic->PageAssociationSize;
		Seg[i].PageAssociate = SymbolDic->PageAssociate;
		Seg[i].retainMe = SymbolDic->retainMe;;
		Seg[i].numReferSeg = SymbolDic->numReferSeg;
		Seg[i].pRetainBits=SymbolDic->pRetainBits;
		Seg[i].pReferSegNo = SymbolDic->pReferSegNo;
		Seg[i].PageAssociate = SymbolDic->PageAssociate;
		Seg[i].SegmentDataLength=SymbolDic->SegmentDataLength;
		Seg[i].PageInfo = NULL;
		Seg[i].SymbolDic = SymbolDic;
		Seg[i].TextRegion=NULL;
		Seg[i].PatternDic=NULL;
		Seg[i].HalfRegion=NULL;
		Seg[i].GenericRegion=NULL;
		Seg[i].EndPage=NULL;
		i++;
		if( SymbolDic->Child != NULL )
			SymbolDic = SymbolDic->Child;
		else
			Flag=0;
	}

	if(TextRegion!=NULL){
		TextRegion = TextRegionSearch( TextRegion, 0);
		Flag=1;
		while(TextRegion != NULL && Flag ){
			Seg[i].SegNo=i;
			Seg[i].SegmentType = TextRegion->SegmentType;
			Seg[i].PageAssociationSize = TextRegion->PageAssociationSize;
			Seg[i].PageAssociate = TextRegion->PageAssociate;
			Seg[i].retainMe = TextRegion->retainMe;;
			Seg[i].numReferSeg = TextRegion->numReferSeg;
			Seg[i].pRetainBits=TextRegion->pRetainBits;
			Seg[i].pReferSegNo = TextRegion->pReferSegNo;
			Seg[i].PageAssociate = TextRegion->PageAssociate;
			Seg[i].SegmentDataLength=TextRegion->SegmentDataLength;
			Seg[i].PageInfo = NULL;
			Seg[i].SymbolDic = NULL;
			Seg[i].TextRegion = TextRegion;
			Seg[i].PatternDic=NULL;
			Seg[i].HalfRegion=NULL;
			Seg[i].GenericRegion=NULL;
			Seg[i].EndPage=NULL;
			i++;
			if(TextRegion->Child!=NULL)
				TextRegion = TextRegion->Child;
			else
				Flag=0;
		}
	}

	if(GenRegion!=NULL){
		GenRegion = GenRegionSearch( GenRegion, 0);
		Flag=1;
		while( GenRegion != NULL && Flag ){
			Seg[i].SegNo=i;
			Seg[i].SegmentType         = GenRegion->SegmentType;
			Seg[i].PageAssociationSize = GenRegion->PageAssociationSize;
			Seg[i].PageAssociate       = GenRegion->PageAssociate;
			Seg[i].retainMe            = GenRegion->retainMe;;
			Seg[i].numReferSeg         = GenRegion->numReferSeg;
			Seg[i].pRetainBits         = GenRegion->pRetainBits;
			Seg[i].pReferSegNo         = GenRegion->pReferSegNo;
			Seg[i].PageAssociate       = GenRegion->PageAssociate;
			Seg[i].SegmentDataLength   = GenRegion->SegmentDataLength;
			Seg[i].PageInfo = NULL;
			Seg[i].SymbolDic = NULL;
			Seg[i].TextRegion =NULL;
			Seg[i].PatternDic=NULL;
			Seg[i].HalfRegion=NULL;
			Seg[i].GenericRegion       = GenRegion;
			Seg[i].EndPage=NULL;
			i++;
			if( GenRegion->Child != NULL )
				GenRegion = GenRegion->Child;
			else
				Flag=0;
		}
	}

	Seg[i].SegNo=i;
	Seg[i].SegmentType = Jb2Param->EndPage->SegmentType;
	Seg[i].PageAssociationSize = Jb2Param->EndPage->PageAssociationSize;
	Seg[i].PageAssociate = Jb2Param->EndPage->PageAssociate;
	Seg[i].retainMe = Jb2Param->EndPage->retainMe;;
	Seg[i].numReferSeg = Jb2Param->EndPage->numReferSeg;
	Seg[i].pRetainBits=Jb2Param->EndPage->pRetainBits;
	Seg[i].pReferSegNo = Jb2Param->EndPage->pReferSegNo;
	Seg[i].PageAssociate = Jb2Param->EndPage->PageAssociate;
	Seg[i].SegmentDataLength=Jb2Param->EndPage->SegmentDataLength;
	Seg[i].PageInfo = NULL;
	Seg[i].SymbolDic = NULL;
	Seg[i].TextRegion =NULL;
	Seg[i].PatternDic=NULL;
	Seg[i].HalfRegion=NULL;
	Seg[i].GenericRegion =NULL;
	Seg[i].EndPage = Jb2Param->EndPage;

	return	Seg;
}

void	*Jb2_ImageBit1ToChar(struct Image_s *rImage)
{
	struct	Image_s *Image=NULL;
	byte4	width, height, s_col1step, t_col1step, s_row1step;
	byte4	i,j,k;
	uchar	*Ds_TS, *Dt_TS, *Ds_, *Dt_;
	uchar	tempD;

	width =rImage->tbx1-rImage->tbx0;
	height=rImage->tby1-rImage->tby0;
	Image = ImageCreate( Image, width, height, 0, width, 0,height, CHAR);
	s_col1step = rImage->col1step;
	s_row1step = rImage->row1step;
	t_col1step = Image->col1step;

	if(rImage->type==BIT1){
		Ds_TS=(uchar *)rImage->data;
		Dt_TS=(uchar *)Image->data;
		for( j=0 ; j<height ; j++, Ds_TS = &Ds_TS[s_col1step], Dt_TS=&Dt_TS[t_col1step] ){
			Ds_ = Ds_TS;
			Dt_ = Dt_TS;
			for(i=0, k=0 ; i<width ; i++, k--){
				if(!k){
					if(rImage->MaxValue==-1)
						tempD = *Ds_;
					else
						tempD = ~(*Ds_);
					k=8;
					++Ds_;
				}
				Dt_[i]= ((tempD>>(k-1)) &1 );
			}
		}
		Image->MaxValue=-1;//rImage->MaxValue;
	}
	else if(rImage->type==CHAR){
		Ds_TS=(uchar *)rImage->data;
		Dt_TS=(uchar *)Image->data;
		if(s_row1step==3){
			for( j=0 ; j<height ; j++, Ds_TS = &Ds_TS[s_col1step], Dt_TS=&Dt_TS[t_col1step] ){
				Ds_ = Ds_TS;
				Dt_ = Dt_TS;
				for( i=0 ; i<width ; i++ )
					Dt_[i]= ((Ds_[i*s_row1step] | Ds_[i*s_row1step+1] | Ds_[i*s_row1step]) ? 1:0) ;
			}
		}
		else{
			for( j=0 ; j<height ; j++, Ds_TS = &Ds_TS[s_col1step], Dt_TS=&Dt_TS[t_col1step] ){
				Ds_ = Ds_TS;
				Dt_ = Dt_TS;
				for( i=0 ; i<width ; i++ )
					Dt_[i]= ( Ds_[i*s_row1step]? 1:0 );
			}
		}
	}
	return	(void *)Image;
}
