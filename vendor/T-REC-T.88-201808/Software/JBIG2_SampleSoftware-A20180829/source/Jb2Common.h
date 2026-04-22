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




#ifndef	_JB2COMMON_H_
#define _JB2COMMON_H_

#include	"ImageUtil.h"


enum JB2_SEGMENT_TYPE
{
	SYMBOL_DICTIONARY                          =0x00,
	INTERMEDIATE_TEXT_REGION                   =0x04,
	IMMEDIATE_TEXT_REGION                      =0x06,
	IMMEDIATE_LOSLESS_TEXT_REGION              =0x07,
	PATTERN_DICTIONARY                         =0x10,
	INTERMIDIATE_HALFTONE_REGION               =0x14,
	IMMEDIATE_HALFTONE_REGION                  =0x16,
	IMMEDIATE_LOSLESS_HALFTONE_REGION          =0x17,
	INTERMEDIATE_GENERIC_REGION                =0x24,
	IMMEDIATE_GENERIC_REGION                   =0x26,//38,
	IMMEDIATE_LOSLESS_GENERIC_REGION           =0x27,//39,
	INTERMEDIATE_GENERIC_REFINMENT_REGION      =0x28,//40,
	IMMEDIATE_GENERIC_REFINMENT_REGION         =0x2a,//42,
	IMMEDIATE_LOSLESS_GENERIC_REFINMENT_REGION =0x2b,//43,
	PAGE_INFORMATION                           =0x30,//48,
	END_OF_PAGE                                =0x31,//49,
	END_OF_STRIPE                              =0x32,//50,
	END_OF_FILE                                =0x33,//51,
	PROFILES                                   =0x34,//52,
	TABLES                                     =0x35,//53,
	COLOUR_PALETTE                             =0x36,//54,
	EXTENSION                                  =0x3e //62
};

static uchar DefaultColorSet[96]={
	  0,  0,  0,//0
	128,128,128,//1
	192,192,192,//2
	255,255,255,//3
	255,  0,  0,//4
	  0,255,  0,//5
	  0,  0,255,//6
	255,255,  0,//7
	  0,255,255,//8
	255,  0,255,//9
	128,  0,  0,//10
	  0,128,  0,//11
	  0,  0,128,//12
	128,128,  0,//13
	  0,128,128,//14
	128,  0,128,//15
	255,168,  0,//16
	204,204,  0,//17
	153,  0,  0,//18
	  0,204,  0,//19
	  0,153,  0,//20
	204,204,  0,//21
	153,153,  0,//22
	102,  0,  0,//23
	  0,  0,204,//24
	  0,  0,153,//25
	204,  0,204,//26
	153,  0,153,//27
	  0,204,204,//28
	  0,153,153,//29
	102,102,102,//30
	153,153,153	//31
};



//SegemntHeaderFlag
#define	DefferedNonRetain		0x80
#define	PageAssociation_Size	0x40
#define	Segment_Type			0x3f


/***********************************************************************************************************************/
/*7.4.2.1参照 "SymbolDictionary" ***************************************************************************************/
/***********************************************************************************************************************/
//7.4.2.1.1参照 "SymbolDictionaryFlags"
//#define	Sdr_Template				0x1000
//#define	Sd_Template					0x0c00
//#define	Bitmap_CodingContextRetain	0x0200
//#define	Bitmap_CodingContexUsed		0x0100
//#define	Sd_HuffAgginstSelect		0x0080
//#define	Sd_HuffBmsizeSelect			0x0040
//#define	Sd_HuffDwSelect				0x0030
//#define	Sd_HuffDhSelect				0x000c
//#define	Sd_RefAgg					0x0002
//#define	Sd_Huff						0x0001
/***********************************************************************************************************************/
/***********************************************************************************************************************/

/***********************************************************************************************************************/
/*7.4.3.1 "Text Region" "Intermediate Text Region" "Immediate Text Region" "Immediate Lossless Text Region" ************/
/***********************************************************************************************************************/
//TextRegionSegmentFlags(7.4.3.1.1)
#define Sbr_Template	0x8000
#define	Sb_DsOffset	0x7c00
#define	Sb_DefPixel	0x0200
#define	Sb_CombOp	0x0180
#define	TransPosed	0x0040
#define	Ref_Corner	0x0030
#define	Log_SbStrips	0x000c
#define	Sb_Refine	0x0002
#define	Sb_Huff		0x0001
#define	Sb_CombOR	0x0000
#define	Sb_CombAND	0x0080
#define	Sb_CombXOR	0x0100
#define	Sb_CombXNOR	0x0180
#define	Jb2_BottomLeft	0
#define	Jb2_TopLeft		1
#define	Jb2_BottomRight	2
#define	Jb2_TopRight	3


//TextRegionSegmentHuffmanFlags(7.4.3.1.2)
#define SbHuffRSize	0x4000
#define	SbHuffRdY	0x3000
#define	SbHuffRdX	0x0c00
#define	SbHuffRdH	0x0300
#define	SbHuffRdW	0x00c0
#define	SbHuffDT	0x0030
#define	SbHuffDS	0x000c
#define	SbHuffFS	0x0003

#define	JBIG2_OR	0
#define	JBIG2_AND	1
#define	JBIG2_XOR	2
#define	JBIG2_XNOR	3

#define	JBIG2_BOTTOM_LEFT 0
#define	JBIG2_TOP_LEFT 1
/***********************************************************************************************************************/
/***********************************************************************************************************************/

#define	IAAI	0x10000
#define	IADH	0x10200
#define	IADS	0x10400
#define	IADT	0x10600
#define	IADW	0x10800
#define	IAEX	0x10A00
#define	IAFS	0x10C00
#define	IAID	0x10E00
#define	IAIT	0x11000
#define	IARDH	0x11200
#define	IARDW	0x11400
#define	IARDX	0x11600
#define	IARDY	0x11800
#define	IARI	0x11A00
#define	Number_CX	0x12000

enum FILE_ACCESS_MODE
{
	RANDOM = 0,
	SEQUENTIAL = 1
};

enum NUM_PAGE_KNOWN
{
	KNOWN = 0,
	UNKNOWN = 1
};

static uchar JB2_FILE_HEADER_ID[8] ={ 0x97, 'J', 'B', '2', 0x0d, 0x0a, 0x1a, 0x0a };

typedef struct Jbig2Parameter_s
{
	byte4	NumberOfSegments;
	struct	ImageChain_s *ImagePage;
	struct	Jb2SegmentHeader_s			*Seg;
	struct	PageInformationSegment_s	*PageInfo;
	struct	EndOfPageSegment_s			*EndPage;
	struct	SymbolDictionarySegment_s	*SymbolDic;
	struct	TextRegionSegment_s			*TextRegion;
	struct	PatternDictionarySegment_s	*PatternDic;
	struct	HalftoneRegionSegment_s		*HalfRegion;
	struct	ImageChain_s *ImageTxt;
	struct	Jb2_ImageChain_s *ImageSym;
	struct	ImageChain_s *ImageHaf;
	struct	ImageChain_s *ImagePat;
	struct	GenericRegionSegment_s		*GenRegion;
	struct	ImageChain_s *ImageGen;
} Jbig2Parameter_t;

typedef	struct	Jb2_ImageChain_s{
	struct	Image_s *Image;
	struct	Jb2_ImageChain_s *child;
	struct	Jb2_ImageChain_s *parent;
	byte4	numChains;
	byte4	ID;
	byte4	Cur_s;	//
	byte4	Cur_T;	//
	byte4	RefID;	//ID of Image for Refinement
	byte4	RDx;
	byte4	RDy;
	uchar	FileID;
} Jb2_ImageChain_t;

typedef struct Jb2SegmentHeader_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;
	struct	PageInformationSegment_s *PageInfo;
	struct	SymbolDictionarySegment_s *SymbolDic;
	struct	TextRegionSegment_s *TextRegion;
	struct	PatternDictionarySegment_s *PatternDic;
	struct	HalftoneRegionSegment_s *HalfRegion;
	struct	GenericRegionSegment_s *GenericRegion;
	struct	EndOfPageSegment_s *EndPage;
} Jb2SegmentHeader_t;

//7.4.2 "SymbolDictionarySegment"
typedef struct SymbolDictionarySegment_s
{
	ubyte4	SegNo;
	byte4	numHeight;
	byte4	*Height;
	byte4	*numWidth;
	byte4	**Width;
	byte4	RefAggnInst;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;
	uchar	Huff;
	uchar	RefinementAggregate;	//Refinement or Aggregate 1:ON 0:OFF
	uchar	Huff_DH_Selection;
	uchar	Huff_DW_Selection;
	uchar	Huff_BmSize_Selection;
	uchar	Huff_Agginst_Selection;
	uchar	BitmapCodingContextUsed;
	uchar	BitmapCodingContextRetained;
	uchar	BmSize;
	char	Template;			//Only when arithMetric
	char	RefTemplate;//
	char	ATX1;				//SymbolDictionaryATFlags
	char	ATY1;				//SymbolDictionaryATFlags
	char	ATX2;				//SymbolDictionaryATFlags
	char	ATY2;				//SymbolDictionaryATFlags
	char	ATX3;				//SymbolDictionaryATFlags
	char	ATY3;				//SymbolDictionaryATFlags
	char	ATX4;				//SymbolDictionaryATFlags
	char	ATY4;				//SymbolDictionaryATFlags
	char	RefATX1;			//SymbolDictionary Refinement AT Flags
	char	RefATY1;			//SymbolDictionary Refinement AT Flags
	char	RefATX2;			//SymbolDictionary Refinement AT Flags
	char	RefATY2;			//SymbolDictionary Refinement AT Flags
	byte4	SdNumExSyms;
	byte4	SdNumNewSyms;
	byte4	numSymbol_File;
	byte4	numSymbol_Page;
	struct	Jb2_ImageChain_s *ImageSym;
	struct	SymbolDictionarySegment_s *Parent;
	struct	SymbolDictionarySegment_s *Child;
	byte4	numberSymbolDic;
} SymbolDicSegDataPart_t;

//ISO/IEC 14492 7.4.3(P56) This structer is common used in "IntermediateTextRegion","ImmediateTextRegion" and "ImmediateLosslessTextRegion".
typedef	struct	TextRegionSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;
	ubyte4	RegionSegmentBitmapWidth;
	ubyte4	RegionSegmentBitmapHeight;
	ubyte4	RegionSegmentXlocation;
	ubyte4	RegionSegmentYlocation;
	uchar	ExternalCombinationOperator;
	uchar	Huff;
	uchar	Refine;
	uchar	LogSbStrips;
	uchar	RefCorner;
	uchar	Transposed;
	uchar	ColourExtFlag;
	byte4	*Col;
	uchar	numCmpts;
	uchar	CmptsL;
	byte4	num_Val;
	uchar	SbCombOp;
	uchar	SbDefPixel;
	uchar	SbDsOffset;
	uchar	SbrTemplate;
	uchar	HuffFsSelection;
	uchar	HuffDsSelection;
	uchar	HuffDtSelection;
	uchar	HuffRdWSelection;
	uchar	HuffRdHSelection;
	uchar	HuffRdXSelection;
	uchar	HuffRdYSelection;
	uchar	HuffRSizeSelection;
	uchar	RefATX1;				//SymbolDictionaryATFlags
	uchar	RefATY1;				//SymbolDictionaryATFlags
	uchar	RefATX2;				//SymbolDictionaryATFlags
	uchar	RefATY2;				//SymbolDictionaryATFlags
	byte4	SbNumInstances;
	byte4	*ID;
	byte4	*Ly;
	byte4	*Lx;
	byte4	*RefDx;
	byte4	*RefDy;
	struct	Jb2_ImageChain_s *ImageSymT;
	struct	TextRegionSegment_s *Parent;
	struct	TextRegionSegment_s *Child;
} TextRegionSegment_t;

//ISO/IEC 14492 7.4.4(P66) "PatternDictionarySegment"
typedef struct PatternDictionarySegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;

	uchar	HdMMR;
	uchar	HdTemplate;
	uchar	HDPW;
	uchar	HDPH;
	ubyte4	GrayMax;
	byte4	numPattern;
} PatternDicSegDataPart_t;

//ISO/IEC 14492 7.4.5(P67) This structer is common used in "IntermediateHalftoneRegion","ImmediateHalftoneRegion" and "ImmediateLosslessHalftoneRegion"
typedef struct HalftoneRegionSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;

	ubyte4	RegionSegmentBitmapWidth;		//RegionSegmentInfoField  Common in the all Region Segment ISO/IEC 14492 7.4.1(P50)
	ubyte4	RegionSegmentBitmapHeight;		//RegionSegmentInfoField
	ubyte4	RegionSegmentXlocation;			//RegionSegmentInfoField
	ubyte4	RegionSegmentYlocation;			//RegionSegmentInfoField
	uchar	ExternalCombinationOperator;	//RegionSegmentInfoField

	uchar	MMR;
	uchar	Template;
	uchar	EnableSkip;
	uchar	CombOp;
	uchar	HDefPixel;
	ubyte4	HGW;
	ubyte4	HGH;
	ubyte4	HGX;
	ubyte4	HGY;
	ubyte2	HRX;
	ubyte2	HRY;
} HalftoneRegionSegDataPart_t;

//ISO/IEC 14492 7.4.6(P70) This structer is common used in "IntermediateGenericRegion" "ImmediateGenericRegion" "ImmediateLosslessRegion".
typedef	struct	GenericRegionSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;

	ubyte4	RegionSegmentBitmapWidth;		//RegionSegmentInfoField  Common in the all Region Segment ISO/IEC 14492 7.4.1(P50)
	ubyte4	RegionSegmentBitmapHeight;		//RegionSegmentInfoField　Region
	ubyte4	RegionSegmentXlocation;			//RegionSegmentInfoField　Region
	ubyte4	RegionSegmentYlocation;			//RegionSegmentInfoField　Region
	uchar	ExternalCombinationOperator;	//RegionSegmentInfoField　Region
	uchar	MMR;
	uchar	ExtTemplate;
	uchar	Template;
	uchar	TpGDon;
	uchar	ColourExtFlag;
	char	ATX1;				//SymbolDictionaryATFlags
	char	ATY1;				//SymbolDictionaryATFlags
	char	ATX2;				//SymbolDictionaryATFlags
	char	ATY2;				//SymbolDictionaryATFlags
	char	ATX3;				//SymbolDictionaryATFlags
	char	ATY3;				//SymbolDictionaryATFlags
	char	ATX4;				//SymbolDictionaryATFlags
	char	ATY4;				//SymbolDictionaryATFlags
	char	ATX5;				//SymbolDictionaryATFlags
	char	ATY5;				//SymbolDictionaryATFlags
	char	ATX6;				//SymbolDictionaryATFlags
	char	ATY6;				//SymbolDictionaryATFlags
	char	ATX7;				//SymbolDictionaryATFlags
	char	ATY7;				//SymbolDictionaryATFlags
	char	ATX8;				//SymbolDictionaryATFlags
	char	ATY8;				//SymbolDictionaryATFlags
	char	ATX9;				//SymbolDictionaryATFlags
	char	ATY9;				//SymbolDictionaryATFlags
	char	ATX10;				//SymbolDictionaryATFlags
	char	ATY10;				//SymbolDictionaryATFlags
	char	ATX11;				//SymbolDictionaryATFlags
	char	ATY11;				//SymbolDictionaryATFlags
	char	ATX12;				//SymbolDictionaryATFlags
	char	ATY12;				//SymbolDictionaryATFlags
	struct	GenericRegionSegment_s *Parent;
	struct	GenericRegionSegment_s *Child;
}GenericRegionSegment_t;


//ISO/IEC 14492 7.4.8(P73) "PageInfomationSegment"
typedef	struct	PageInformationSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;

	byte4	PageBitmapWidth;
	byte4	PageBitmapHeight;
	byte4	PageXResolution;
	byte4	PageYResolution;
	uchar	PageEventuallyLossless;
	uchar	PageMightContainRefinements;
	uchar	PageDefaultPixelValue;
	uchar	PageDefaultCombinationOperator;
	uchar	PageRequiersAuxllaryBuffers;
	uchar	PageCombinationOperatorOverRidden;
	uchar	PageStriped;
	byte2	MaximumStripeSize;
	uchar	ColorExtFlag;			//AMD3 Page might contain coloured Segment
} PageInformationSegment_t;

//ISO/IEC 14492 7.4.9(P73) "EndOfPageSegment"
typedef	struct	EndOfPageSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;


} EndOfPageSegment_t;

//ISO/IEC 14492 7.4.3(P56) This structer is common used in "IntermediateTextRegion","ImmediateTextRegion" and "ImmediateLosslessTextRegion".
typedef	struct	ColourPaletteSegment_s
{
	ubyte4	SegNo;
	uchar	PageAssociationSize;
	uchar	SegmentType;
	uchar 	retainMe;
	ubyte4	numReferSeg;
	uchar 	*pRetainBits;
	ubyte4	*pReferSegNo;
	ubyte4	PageAssociate;
	byte4	SegmentDataLength;
	uchar	FlagfieldContinuesToNextByte;
	uchar	ColourSpace;	//0:RGB 1:sRGB 2:adobe rgb others:reserved
	uchar	NumCompts;		//Number of color components.
	uchar	ComptsLength;	//1:uchar 2:byte2 4:byte4
	byte4	ComptsValue;

} ColourPaletteSegment_t;

struct StreamChain_s *JBIG2_EncMain( struct StreamChain_s *str, struct Jbig2Parameter_s *Param, struct ImageChain_s *ImagePage );
void	*JBIG2_DecMain( struct StreamChain_s *str);
struct Jbig2Parameter_s *Jb2ParamInit( char *fname_ini, struct ImageChain_s *ImagePage, char enc_dec);
struct Jb2SegmentHeader_s *SegmentCreate( struct Jbig2Parameter_s *Jb2Param );

struct	SymbolDictionarySegment_s *SymbolDicSearch(struct SymbolDictionarySegment_s *SymbolDic, char SegNo);
struct	TextRegionSegment_s *TextRegionSearch(struct TextRegionSegment_s *TextRegion, char SegNo);
struct	GenericRegionSegment_s *GenRegionSearch(struct GenericRegionSegment_s *GenRegion, char SegNo);

struct	SymbolDictionarySegment_s *SymbolDicCreate( struct SymbolDictionarySegment_s *SymbolDic0 );
struct	TextRegionSegment_s *TextRegionCreate( struct TextRegionSegment_s *TextRegion0 );
struct	GenericRegionSegment_s *GenRegionCreate( struct GenericRegionSegment_s *GenRegion0 );

void	SymbolDic_Initiate(struct SymbolDictionarySegment_s *SymbolDic);
void	TextRegion_Initiate(struct	TextRegionSegment_s *TextRegion);
void	GenRegion_Initiate(struct GenericRegionSegment_s *GenRegion);


struct ImageChain_s *SegmentDecode( struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, byte4 NumberOfSegments, uchar *ColorExtFlag);
struct StreamChain_s *SegmentEncode( struct StreamChain_s *str, struct Jbig2Parameter_s *Jb2Param, struct Jb2SegmentHeader_s *Seg, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, struct ImageChain_s *ImagePage );
struct Jb2_ImageChain_s *Jb2_ImageChainChildSearch( struct Jb2_ImageChain_s *ImageC );
struct Jb2_ImageChain_s *Jb2_ImageChainParentSearch( struct Jb2_ImageChain_s *ImageC );
struct Jb2_ImageChain_s *Jb2_ImageChainSearch( struct Jb2_ImageChain_s *ImageC, byte4 numChain);
struct Jb2_ImageChain_s *Jb2_ImageChainCreate( struct Jb2_ImageChain_s *Parent );


void	*Jb2_ImageBit1ToChar(struct Image_s *rImage);
void	Jbig2_ImageMarg(struct Image_s *S, struct Image_s *T, uchar CombOP, byte4 Cur_T, byte4 Cur_S, uchar RefCorner, uchar ColourExtFlag, byte4 *Col );
byte4	SegmentNumCount( struct StreamChain_s *str);
byte4	*B3_Sort( byte4 *B3_L, byte4 numCode);
void	B3_RunCodeCreate( byte4 *RunCode_V, byte4 *RunCode_L, byte4 *RunCode_O, byte4 numCode );
void	SymbolID_Create( struct StreamChain_s *str, byte4 *SymbolID_V, byte4 *SymbolID_L, byte4 *SymbolID_O, byte4 *RunCode_V, byte4 *RunCode_L, byte4 *RunCode_O, ubyte4 numSymbol);

//SegmentDecode
struct	ImageChain_s *HalftoneRegionSegmentDec( struct ImageChain_s *ImagePage, struct ImageChain_s *ImageHaf, struct ImageChain_s *ImagePat, struct StreamChain_s *str, struct HalftoneRegionSegment_s *HalfRegion, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 numPattern, uchar ColorExtFlag );

void TextRegionDec_Huff_Kernel( struct Image_s *Image, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, uchar sHuff, uchar LogSbStrips, uchar RefCorner, uchar CombOp, uchar DsOffset, byte4 SbNumInstances, byte4 numCode, uchar HuffDtSelection, uchar HuffFsSelection, uchar HuffDsSelection, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 MQ_Eaddr, ubyte4 numSymbol, uchar ColourExtFlag, byte4 *Col);
void	SymbolAggregate_MQDec( struct Image_s *Image, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, uchar RefinementAggregate, uchar StripT0, uchar RefCorner, uchar CombOp, uchar DsOffset, byte4 SbNumInstances, byte4 numCode, struct mqcodec_s *codec, byte4 MQ_Eaddr, ubyte4 numSymbol, uchar Template, char ATX1, char CTY1, char ATX2, char ATY2, uchar ColourExtFlag, byte4 *Col);
byte4	TextRegionSegmentDec( struct ImageChain_s *ImagePage, /*struct ImageChain_s *ImageTxt,*/ struct Jb2_ImageChain_s *ImageSym, struct TextRegionSegment_s *TextRegion, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, ubyte4 numSymbol/*, uchar temptemp*/ );
struct	StreamChain_s *TextRegionSegmentEnc( struct StreamChain_s *str, struct TextRegionSegment_s *TextRegion, struct Jb2_ImageChain_s *ImageSym, struct ImageChain_s *ImagePage, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, ubyte4 numSymbol );

struct	ImageChain_s *GenericRegionSegmentDec( struct ImageChain_s *ImagePage, struct ImageChain_s *ImageGen, struct StreamChain_s *str, struct GenericRegionSegment_s *GenRegion, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, uchar ColorExtFlag );
struct	StreamChain_s *ImmediateLosslessGenericRegionSegmentEnc( struct GenericRegionSegment_s *GenRegion, struct StreamChain_s *str, struct ImageChain_s *ImagePage, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec );

void	EndOfPageSegmentDec( struct ImageChain_s *ImagePage, byte4 PageNumber );
struct	StreamChain_s *EndOfPageSegmentEnc( struct Jbig2Parameter_s *Jb2Param, struct StreamChain_s *str );

void ColourPaletteSegmentDec( struct ColourPaletteSegment_s *ColourPalette, struct StreamChain_s *str );

struct	ImageChain_s *PageInformationSegmentDec(struct ImageChain_s *Image_Page, struct PageInformationSegment_s *PageInfo, struct StreamChain_s *str, uchar *ColorExtFlag );
struct	StreamChain_s *PageInformationSegmentEnc( struct	PageInformationSegment_s *PageInfo, struct StreamChain_s *str );

struct	Jb2_ImageChain_s *SymbolDictionarySegmentDec( byte4 *SymCount, struct Jb2_ImageChain_s *ImageSym, struct ImageChain_s *ImageTxt, struct SymbolDictionarySegment_s *SymbolDic, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, char PageFlag );
struct	StreamChain_s *SymbolDictionarySegmentEnc( struct SymbolDictionarySegment_s *SymbolDic, struct Jb2_ImageChain_s *ImageSym, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec, byte4 *SymbolCount, char PageFlag);

struct ImageChain_s *PatternDictionarySegmentDec( struct ImageChain_s *ImagePat, struct StreamChain_s *str, struct	PatternDictionarySegment_s *PatternDic, struct Jb2HuffmanTable_s *Huff, struct mqcodec_s *codec );
#endif

