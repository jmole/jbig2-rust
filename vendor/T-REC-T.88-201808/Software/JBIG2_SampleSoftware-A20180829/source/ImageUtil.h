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


#ifndef ImageUtil_h
#define ImageUtil_h


#include	"asmasm.h"

#ifndef	BIG_ENDIAN
#define	BIG_ENDIAN		1
#endif

#ifndef LITTLE_ENDIAN
#define	LITTLE_ENDIAN	0
#endif

#define	LSBFirst		1
#define	MSBFirst		0

//Stream type for StreamChain_s.
#define	JPEG		1		//1byte discard after "FF"
#define	JPEG2000	2		//1bit discard after "FF"
#define	JPEGXR		3
#define	JPEG_KAWAJIRI	4
#define	JPEG_XT		5
#define	JBIG2		6
#define	NoDiscard	7		//No discard after "FF"



#define BUF_LENGTH 0x8000
#define TRUE NULL
#define FALES -1//NULL
#define ZERO 0

#define UP 1
#define DOWN 0

#define	ON		1
#define	OFF		0

#define	ENC		1
#define	DEC		0
#define BMP 0x01
#define PGM 0x02
#define PNM 0x04
#define PGX 0x08
#define	TIF	0x10
#define	RAW	0x30



//Tiff Tag
#define	TagImageWidth			0x0100
#define	TagImageHeight			0x0101
#define	TagBitsPerSample		0x0102	//258
#define	TagCompression			0x0103	//259
#define	TagPhotometric			0x0106	//262
#define	TagDocumentName			0x010d
#define	TagImageDiscription		0x010e
#define	TagEquipmentMake		0x010f
#define	TagEquipmentModel		0x0110
#define	TagStripOffsets			0x0111	//273
#define	TagSamplesPerPixel		0x0115	//277
#define	TagRowsPerStrip			0x0116	//278
#define	TagStripByteCounts		0x0117	//279
#define	TagMinValue				0x0118	//280
#define	TagMaxValue				0x0119	//281
#define	TagXResolution			0x011a	//282
#define	TagYResolution			0x011b	//283
#define	TagProgressive			0x011c	//284
#define	TagPageName				0x011d
#define	TagResolutionUnit		0x0128	//296
#define	TagPageNumber			0x0129
#define	TagSoftwareNameVersion	0x0131
#define	TagDateTile				0x0132
#define	TagArtistName			0x013b
#define	TagHostComputer			0x013c
#define	TagInkSet				0x014c	//332
#define	TagCopyrightNotice		0x8298
#define	TagColorSpace			0xa001
#define	TagPixelFormat			0xbc01
#define	TagSpatialXfrmPrimary	0xbc02
#define	TagUlong				0xbc04
#define	TagPTMColorInfo			0xbc05
#define	TagProfileInfo			0xbc06
#define	TagImageWidthJXR		0xbc80
#define	TagImageHeightJXR		0xbc81
#define	TagWidthResolution		0xbc82
#define	TagHeightResolution		0xbc83
#define	TagStripOffsetsJXR		0xbcc0
#define	TagStripByteCountsJXR	0xbcc1
#define	TagAlphaOffset			0xbcc2
#define	TagAlphaByteCount		0xbcc3
#define	TagImageBandPresence	0xbcc4
#define	TagAlphaBandPresence	0xbcc5
#define	TagPaddingData			0xea1c


//PixelFormat 0xC93C‚ªŒ‡”Ô
#define	PixFmt1								0x24C3DD6F
#define	PixFmt2								0x034EFE4B
#define	PixFmt3								0xB1853D77
#define	PixFmt4								0x768DC9

#define PixFmt_BlackWhite					0x05
#define PixFmt_8bppGray						0x08
#define PixFmt_16bppBGR555					0x09
#define PixFmt_16bppBGR565					0x0A
#define PixFmt_16bppGray					0x0B
#define PixFmt_24bppBGR						0x0C
#define PixFmt_24bppRGB						0x0D
#define PixFmt_32bppBGR						0x0E
#define PixFmt_32bppBGRA					0x0F
#define PixFmt_32bppPBGRA					0x10
#define PixFmt_32bppGrayFloat				0x11
#define PixFmt_48bppRGBFixedPoint			0x12
#define PixFmt_16bppGrayFixedPoint			0x13
#define PixFmt_32bppBGR101010				0x14
#define PixFmt_48bppRGB						0x15
#define PixFmt_64bppRGBA					0x16
#define PixFmt_64bppPRGBA					0x17
#define PixFmt_96bppRGBFixedPoint			0x18
#define PixFmt_128bppRGBAFloat				0x19
#define PixFmt_128bppPRGBAFloat				0x1A
#define PixFmt_128bppRGBFloat				0x1B
#define PixFmt_32bppCMYK					0x1C
#define PixFmt_64bppRGBAFixedPoint			0x1D
#define PixFmt_128bppRGBAFixedPoint			0x1E
#define PixFmt_64bppCMYK					0x1F
#define PixFmt_24bpp3Channels				0x20
#define PixFmt_32bpp4Channels				0x21
#define PixFmt_40bpp5Channels				0x22
#define PixFmt_48bpp6Channels				0x23
#define PixFmt_56bpp7Channels				0x24
#define PixFmt_64bpp8Channels				0x25
#define PixFmt_48bpp3Channels				0x26
#define PixFmt_64bpp4Channels				0x27
#define PixFmt_80bpp5Channels				0x28
#define PixFmt_96bpp6Channels				0x29
#define PixFmt_112bpp7Channels				0x2A
#define PixFmt_128bpp8Channels				0x2B
#define PixFmt_40bppCMYKAlpha				0x2C
#define PixFmt_80bppCMYKAlpha				0x2D
#define PixFmt_32bpp3ChannelsAlpha			0x2E
#define PixFmt_40bpp4ChannelsAlpha			0x2F
#define PixFmt_48bpp5ChannelsAlpha			0x30
#define PixFmt_56bpp6ChannelsAlpha			0x31
#define PixFmt_64bpp7ChannelsAlpha			0x32
#define PixFmt_72bpp8ChannelsAlpha			0x33
#define PixFmt_64bpp3ChannelsAlpha			0x34
#define PixFmt_80bpp4ChannelsAlpha			0x35
#define PixFmt_96bpp5ChannelsAlpha			0x36
#define PixFmt_112bpp6ChannelsAlpha			0x37
#define PixFmt_128bpp7ChannelsAlpha			0x38
#define PixFmt_144bpp8ChannelsAlpha			0x39
#define PixFmt_64bppRGBAHalf				0x3A
#define PixFmt_48bppRGBHalf					0x3B
#define PixFmt_32bppRGBE					0x3D
#define PixFmt_16bppGrayHalf				0x3E
#define PixFmt_32bppGrayFixedPoint			0x3F
#define PixFmt_64bppRGBFixedPoint			0x40
#define PixFmt_128bppRGBFixedPoint			0x41
#define PixFmt_64bppRGBHalf					0x42
#define PixFmt_80bppCMYKDIRECTAlpha			0x43
#define PixFmt_12bppYCC420					0x44
#define PixFmt_16bppYCC422					0x45
#define PixFmt_20bppYCC422					0x46
#define PixFmt_32bppYCC422					0x47
#define PixFmt_24bppYCC444					0x48
#define PixFmt_30bppYCC444					0x49
#define PixFmt_48bppYCC444					0x4A
#define PixFmt_48bppYCC444FixedPoint		0x4B
#define PixFmt_20bppYCC420Alpha				0x4C
#define PixFmt_24bppYCC422Alpha				0x4D
#define PixFmt_30bppYCC422Alpha				0x4E
#define PixFmt_48bppYCC422Alpha				0x4F
#define PixFmt_32bppYCC444Alpha				0x50
#define PixFmt_40bppYCC444Alpha				0x51
#define PixFmt_64bppYCC444Alpha				0x52
#define PixFmt_64bppYCC444AlphaFixedPoint	0x53
#define PixFmt_32bppCMYKDIRECT				0x54
#define PixFmt_64bppCMYKDIRECT				0x55
#define PixFmt_40bppCMYKDIRECTAlpha			0x56


#define	CHAR	0x01
#define	BYTE2	0x02
#define	BYTE4	0x04
#define	BIT1	0x10
#define	FLOAT2	0x20
#define	FLOAT4	0x40

//#define	Metric_RGB		0x01
//#define	Metric_BGR		0x02
#define	DD_a		0x61
#define	DD_b		0x62
#define	DD_c		0x63
#define	DD_d		0x64
#define	DD_e		0x65
#define	DD_f		0x66
#define	DD_g		0x67
#define	DD_i		0x69
#define	DD_k		0x6b
#define	DD_l		0x6c
#define	DD_m		0x6d
#define	DD_n		0x6e
#define	DD_o		0x6f
#define	DD_p		0x70
#define	DD_q		0x71
#define	DD_r		0x72
#define	DD_s		0x73
#define	DD_t		0x74
#define	DD_v		0x76
#define	DD_w		0x77
#define	DD_0		0x30
#define	DD_1		0x31
#define	DD_2		0x32
#define	DD__		0x5f
#define	DD_Space	0x20
#define	DD_Plus		0x2b
#define	DD_Minus	0x2d
#define	DD_Point	0x2e
#define	DD_BracketL	0x3c
#define	DD_Equal	0x3d
#define	DD_BracketR	0x3e

#define	METRIC_YONLY		0
#define	METRIC_YUV420		1
#define	METRIC_YUV422		2
#define	METRIC_YUV444		3
#define	METRIC_CMYK			4
#define	METRIC_YUVK			4
#define	METRIC_CMYKdirect	5
#define	METRIC_NCOMPONENT	6
#define	METRIC_RGB			7
#define	METRIC_RGBe			8
#define	METRIC_BGR			9

typedef unsigned char	uchar;
typedef char			byte1;
typedef unsigned short	ubyte2;
typedef short			byte2;
typedef unsigned int	ubyte4;
typedef int				byte4;
//typedef __int64        byte8;
typedef long			byte8;

#ifdef _MSC_VER
typedef	int				INSTAN;		
#else
typedef	long long		INSTAN;
#endif

#define MASK_LENGTH 33
static ubyte4 mask6[MASK_LENGTH-1]={	0x00000000, 0x00000001, 0x00000003, 0x00000007,
										0x0000000f,	0x0000001f,	0x0000003f,	0x0000007f,
										0x000000ff,	0x000001ff,	0x000003ff,	0x000007ff,
										0x00000fff,	0x00001fff,	0x00003fff,	0x00007fff,
										0x0000ffff,	0x0001ffff,	0x0003ffff,	0x0007ffff,
										0x000fffff,	0x001fffff,	0x003fffff,	0x007fffff,
										0x00ffffff,	0x01ffffff,	0x03ffffff,	0x07ffffff,
										0x0fffffff,	0x1fffffff,	0x3fffffff,	0x7fffffff};

static ubyte4 mask5[MASK_LENGTH-1]={	0x00000001, 0x00000002, 0x00000004, 0x00000008,
										0x00000010,	0x00000020,	0x00000040,	0x00000080,
										0x00000100,	0x00000200,	0x00000400,	0x00000800,
										0x00001000,	0x00002000,	0x00004000,	0x00008000,
										0x00010000,	0x00020000,	0x00040000,	0x00080000,
										0x00100000,	0x00200000,	0x00400000,	0x00800000,
										0x01000000,	0x02000000,	0x04000000,	0x08000000,
										0x10000000,	0x20000000,	0x40000000,	0x80000000};

static uchar mask4[9]={ 0x80,0x40,0x20,0x10,0x08,0x04,0x02,0x01,0x00};

static long double mask3[32]={
	0.0000152587890625,	//j=0	//-16
	0.000030517578125,	//j=1	//-15
	0.00006103515625,	//j=2	//-14
	0.0001220703125,	//j=3	//-13
	0.000244140625,	//j=4	//-12
	0.00048828125,	//j=5	//-11
	0.0009765625,	//j=6	//-10
	0.001953125,	//j=7	//-9
	0.00390625,		//j=8	//-8 
	0.0078125,		//j=9	//-7 
	0.015625,		//j=10	//-6
	0.03125,		//j=11	//-5 
	0.0625,			//j=12	//-4
	0.125,			//j=13	//-3
	0.25,			//j=14	//-2
	0.5,			//j=15	//-1
	1.0,			//j=16	//0
	2.0,			//j=17	//1
	4.0,			//j=18	//2
	8.0,			//j=19	//3
	16.0,			//j=20	//4
	32.0,			//j=21	//5
	64.0,			//j=22	//6
	128.0,			//j=23	//7
	256.0,			//j=24	//8
	512.0,			//j=25	//9
	1024.0,			//j=26	//10	
	2048.0,			//j=27	//11
	4096.0,			//j=28	//12
	8192.0,			//j=29	//13
	16384.0,		//j=30	//14
	32768.0			//j=31	//15
};		

static byte4 mask2[MASK_LENGTH]={ 0x00000000,
						0x00000001,0x00000003,0x00000007,0x0000000f,
						0x0000001f,0x0000003f,0x0000007f,0x000000ff,
						0x000001ff,0x000003ff,0x000007ff,0x00000fff,
						0x00001fff,0x00003fff,0x00007fff,0x0000ffff,
						0x0001ffff,0x0003ffff,0x0007ffff,0x000fffff,
						0x001fffff,0x003fffff,0x007fffff,0x00ffffff,
						0x01ffffff,0x03ffffff,0x07ffffff,0x0fffffff,
						0x1fffffff,0x3fffffff,0x7fffffff,-1	};
						
static ubyte4 mask1[MASK_LENGTH]={ 0x00000000,
							0x00000001,	0x00000002,	0x00000004,	0x00000008,
							0x00000010,	0x00000020,	0x00000040,	0x00000080,
							0x00000100,	0x00000200,	0x00000400,	0x00000800,
							0x00001000,	0x00002000,	0x00004000,	0x00008000,
							0x00010000,	0x00020000,	0x00040000,	0x00080000,
							0x00100000,	0x00200000,	0x00400000,	0x00800000,
							0x01000000,	0x02000000,	0x04000000,	0x08000000,
							0x10000000,	0x20000000,	0x40000000,	0x80000000};
	
static uchar mask[9] ={ 0x00,0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80};

//
typedef	struct	LogD_s{
	char	logD[256][256];
	byte4	num_Log;
}LogD_t;

typedef struct Pointer_s{
	char *fname;
	char *fileForm;
	uchar *Stream;
}Pointer_t;

typedef struct StreamChain_s{
	uchar *buf;
	byte4 buf_length;
	byte4 numChains;
	byte4 cur_p;
	byte4 total_p;
	struct StreamChain_s *child;
	struct StreamChain_s *parent;
	uchar lastbyte;
	char bits;
	char	stream_type;
} StreamChain_t;

typedef struct	ImagePlane_s{
	struct	Image_s **Image;
	byte4	numBitPlane;
} ImagePlane_t;

typedef	struct	ImageChain_s{
	struct	Image_s *Image;
	struct	ImageChain_s *child;
	struct	ImageChain_s *parent;
	byte4	numChains;
} ImageChain_t;

typedef	struct	Image_s{
	byte4  *data;
	byte4	tbx0;
	byte4	tbx1;
	byte4	tby0;
	byte4	tby1;
	byte4	width;		//+14
	byte4	height;		//+18
	byte4	col1step;	//+1c
	byte4	row1step;	//+20
	byte4	numData;	//+24
	byte4	*Pdata;		//+28
	uchar	type;		//+2c
	byte4	MaxValue;	//+30
} Image_t;


typedef struct Bmp_s{
	struct BmpInfo_s *BmpInfo;
	uchar *d1;
	byte4 *d;
} Bmp_t;

typedef struct BmpInfo_s{
	uchar B;
	uchar M;
	byte4 value_of_filesize;
	byte4 dumy1;
	byte4 offset;
	byte4 infohead;
	byte4 xpixel;
	byte4 ypixel;
	byte2 plane;
	byte2 colorbits;
	byte4 code_method;
	byte4 size_of_coding;
	byte4 xreso;
	byte4 yreso;
	byte4 colours;
	byte4 import_colour;
	byte4 xwidth;
	uchar *gamma;
	byte4 gamma_length;
	ubyte2 *bit_depth;
	uchar type;
	uchar	Csiz;
} BmpInfo_t;


#ifdef _MSC_VER

#ifdef __cplusplus
extern "C"{
__declspec(dllexport) struct StreamChain_s *StreamChainMake(struct StreamChain_s *s, byte4 buf_length, char stream_type);
}
#endif


struct StreamChain_s *StreamBitWrite(ubyte4 data, byte4 BitLength, struct StreamChain_s *stream, byte4 buf_length);
struct StreamChain_s *StreamBitWriteJ2K(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);
struct StreamChain_s *StreamBitWriteJPG(struct StreamChain_s *str, ubyte4 data, char BitLength);

#if Cplus
struct StreamChain_s *StreamBitWriteJXR(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);
#else
#ifdef __cplusplus
extern "C"{
struct StreamChain_s *StreamBitWriteJXR(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);
}
#endif
#endif

__declspec(dllexport) struct StreamChain_s *StreamChainMake(struct StreamChain_s *s, byte4 buf_length, char stream_type);
struct StreamChain_s *ByteStuffOutJ2K(struct StreamChain_s *s);
struct StreamChain_s *ByteStuffOutJXR(struct StreamChain_s *s);
struct StreamChain_s *ByteStuffOutJPG(struct StreamChain_s *str);
struct StreamChain_s *Stream1ByteWrite(struct StreamChain_s *s, uchar byte, byte4 buf_length);
struct StreamChain_s *Stream2ByteWrite(struct StreamChain_s *s, ubyte2 byte, byte4 buf_length, char mode);
struct StreamChain_s *Stream4ByteWrite(struct StreamChain_s *s, ubyte4 byte, byte4 buf_length, char mode);
struct StreamChain_s *StreamFloatWrite(struct StreamChain_s *str, float byte, byte4 buf_length);
struct StreamChain_s *StreamChainBind(struct StreamChain_s *r, struct StreamChain_s *t);
byte4 Stream4ByteRef(struct StreamChain_s *s, byte4 adr);
byte2 Stream2ByteRef(struct StreamChain_s *s, byte4 adr);
uchar Stream1ByteRef(struct StreamChain_s *s, byte4 adr);
struct StreamChain_s *StreamChainTruncate(struct StreamChain_s *s , byte4 length);
__declspec(dllexport) byte4 StreamToFile(char *fname, struct StreamChain_s *s);
byte4 StreamChainCounter(struct StreamChain_s *s);
struct StreamChain_s *StreamChainChildSearch(struct StreamChain_s *r);
__declspec(dllexport) struct StreamChain_s *StreamChainParentSearch(struct StreamChain_s *r);
byte4 StreamChainDataLength(struct StreamChain_s *r);
void StreamChainDestory(struct StreamChain_s *s);

float Ref_ASCII_Froat(struct StreamChain_s *s, uchar *DFlag);
struct StreamChain_s *Write_ASCII_Froat(struct StreamChain_s *s, float DataD);

float ClampF(float A, float maxmax, float minmin);
byte4 ceil2(byte4 x, byte4 y);
byte4 flor(byte4 x, byte4 y);
byte4	fRounding(double fIn);
byte4	Rounding( byte4 In, byte4 y);
byte4	Umod(byte4 Data, ubyte4 Modulo);
char Ref_1Bit(struct StreamChain_s *s);
byte4 Ref_nBits(struct StreamChain_s *s, uchar bits);
void Ref_StuffBits(struct StreamChain_s *s, uchar mode);
uchar Ref_1Byte(struct StreamChain_s *s);
ubyte2 Ref_2Byte(struct StreamChain_s *s);
ubyte2 Ref_2ByteL(struct StreamChain_s *s);
ubyte4 Ref_4Byte(struct StreamChain_s *s);
float Ref_Float(struct StreamChain_s *str);
ubyte4 Ref_4ByteL(struct StreamChain_s *s);
ubyte2 get2(FILE*fp);
void put2(byte2 d, FILE*fp);
void put4(byte4 e, FILE*fp);
byte4 floorlog2(ubyte4 x);
byte4 ceil2log2(ubyte4 x);

void Print_Bmp(struct Bmp_s *Bmp, char *str);
struct Bmp_s *MakeBmp(char BITs, byte4 xpixel, byte4 ypixel);
byte4 Destroy_Bmp(struct Bmp_s *Bmp);
__declspec(dllexport) void *LoadBmp( char *fname );
__declspec(dllexport) void SaveBmp777(char *fname, struct Image_s *Image);
__declspec(dllexport) void *LoadTif(char *fname);
__declspec(dllexport) void *LoadRAW(char *fname, byte2 numCmpts, byte2 numBitDipth, byte4 width1, byte4 height);
__declspec(dllexport) void *LoadPpm(char *fname);

__declspec(dllexport) uchar *MakeBmpStream(struct Image_s *Image);
__declspec(dllexport) void SaveRAW(byte4 width, byte4 height, /*byte2 numCmpts,*/ char BitDipth, char *fname, struct Image_s *Image);
__declspec(dllexport) void SaveTiff(uchar numCmpts, byte4 width, byte4 xwidth, byte4 height, char BitDipth, char *fname, struct Image_s *Image, uchar PixelFormatData);
__declspec(dllexport) void SavePpm(char *fname, struct Image_s *Image, uchar Bitdipth/*, uchar shift_h*/);
void put_ifd_entry(FILE *fp, ubyte2 tag, byte2 type, byte4 count, ubyte4 offset);
__declspec(dllexport) byte4 SavePgm(struct Bmp_s *Bmp, char *fname, ubyte2 numCmpts);
__declspec(dllexport) void SaveImg(char *fname, struct Image_s *Image );

struct ImageChain_s *ImageChainChildSearch( struct ImageChain_s *ImageC );
struct ImageChain_s *ImageChainParentSearch( struct ImageChain_s *ImageC );
struct ImageChain_s *ImageChainSearch( struct ImageChain_s *ImageC, byte4 numChain);
struct ImageChain_s *ImageChainCreate( struct ImageChain_s *Parent );
struct ImagePlane_s *ImagePlaneCreate( byte4 numBitPlane );
byte4 ImagePlaneDestroy( struct ImagePlane_s *ImagePlane );
struct Image_s* ImageCreate(struct Image_s *image, byte4 width, byte4 height, byte4 tbx0, byte4 tbx1, byte4 tby0, byte4 tby1, char type);
void ImageDestory(struct Image_s *image);
void Print_Image(struct Image_s *image,char *str);
void Print_ImageByte4(struct ImageByte4_s *image, char *str);
void Print_ImageByte2(struct ImageByte2_s *image, char *str);
void Print_ImageChar(struct ImageChar_s *image,char *str);
void	*ImageCharToBit1(struct Image_s *Image);
void	*ImageBit1ToChar(struct Image_s *rImage);

byte4 MatMulti(char *fname1, char *fname2, char *fnameWr);
byte4 floor2(byte4 x, byte4 y);
byte4 RGBtoYUV(struct Image_s *Image, struct Image_s *Stream, uchar ccc, char metric, byte4 offset);
byte4 YUVtoRGB(struct Image_s *RGB, struct Image_s *Y, struct Image_s *U, struct Image_s *V, char metric, byte4 offset);
byte4 DcEnc(struct Image_s *OUT, struct Image_s *IN, byte2 offset, char ccc, char FILTER);
byte4 DcDec(struct Image_s *OUT, struct Image_s *IN, byte4 offset, char foatMode, char ccc);

void ArrangeBigOrder(byte4 *Table, byte4 *Order, byte4 numCode);
void ArrangeLittleOrder(byte4 *Table, byte4 *Order, byte4 numCode);

byte4	Dout4( byte4 *D_, byte4 Addr, byte4 Limit);
byte2	Dout2( byte2 *D_, byte4 Addr, byte4 Limit);
uchar	Dout1( uchar *D_, byte4 Addr, byte4 Limit);


#else
struct StreamChain_s *StreamChainMake(struct StreamChain_s *s, byte4 buf_length, char stream_type);
struct StreamChain_s *StreamBitWrite(ubyte4 data, byte4 BitLength, struct StreamChain_s *stream, byte4 buf_length);
struct StreamChain_s *StreamBitWriteJ2K(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);
struct StreamChain_s *StreamBitWriteJPG(struct StreamChain_s *str, ubyte4 data, char BitLength);
struct StreamChain_s *StreamBitWriteJXR(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length);
struct StreamChain_s *StreamChainMake(struct StreamChain_s *s, byte4 buf_length, char stream_type);
struct StreamChain_s *ByteStuffOutJ2K(struct StreamChain_s *s);
struct StreamChain_s *ByteStuffOutJXR(struct StreamChain_s *s);
struct StreamChain_s *ByteStuffOutJPG(struct StreamChain_s *str);
struct StreamChain_s *Stream1ByteWrite(struct StreamChain_s *s, uchar byte, byte4 buf_length);
struct StreamChain_s *Stream2ByteWrite(struct StreamChain_s *s, ubyte2 byte, byte4 buf_length, char mode);
struct StreamChain_s *Stream4ByteWrite(struct StreamChain_s *s, ubyte4 byte, byte4 buf_length, char mode);
struct StreamChain_s *StreamFloatWrite(struct StreamChain_s *str, float byte, byte4 buf_length);
struct StreamChain_s *StreamChainBind(struct StreamChain_s *r, struct StreamChain_s *t);
byte4 Stream4ByteRef(struct StreamChain_s *s, byte4 adr);
byte2 Stream2ByteRef(struct StreamChain_s *s, byte4 adr);
uchar Stream1ByteRef(struct StreamChain_s *s, byte4 adr);
struct StreamChain_s *StreamChainTruncate(struct StreamChain_s *s , byte4 length);
byte4 StreamToFile(char *fname, struct StreamChain_s *s);
byte4 StreamChainCounter(struct StreamChain_s *s);
struct StreamChain_s *StreamChainChildSearch(struct StreamChain_s *r);
struct StreamChain_s *StreamChainParentSearch(struct StreamChain_s *r);
byte4 StreamChainDataLength(struct StreamChain_s *r);
void StreamChainDestory(struct StreamChain_s *s);
float Ref_ASCII_Froat(struct StreamChain_s *s, uchar *DFlag);
struct StreamChain_s *Write_ASCII_Froat(struct StreamChain_s *s, float DataD);
float ClampF(float A, float maxmax, float minmin);
byte4 ceil2(byte4 x, byte4 y);
byte4 flor(byte4 x, byte4 y);
byte4	fRounding(double fIn);
byte4	Rounding( byte4 In, byte4 y);
byte4	Umod(byte4 Data, ubyte4 Modulo);
char Ref_1Bit(struct StreamChain_s *s);
byte4 Ref_nBits(struct StreamChain_s *s, uchar bits);
void Ref_StuffBits(struct StreamChain_s *s, uchar mode);
uchar Ref_1Byte(struct StreamChain_s *s);
ubyte2 Ref_2Byte(struct StreamChain_s *s);
ubyte2 Ref_2ByteL(struct StreamChain_s *s);
ubyte4 Ref_4Byte(struct StreamChain_s *s);
float Ref_Float(struct StreamChain_s *str);
ubyte4 Ref_4ByteL(struct StreamChain_s *s);
ubyte2 get2(FILE*fp);
void put2(byte2 d, FILE*fp);
void put4(byte4 e, FILE*fp);
byte4 floorlog2(ubyte4 x);
byte4 ceil2log2(ubyte4 x);
void Print_Bmp(struct Bmp_s *Bmp, char *str);
struct Bmp_s *MakeBmp(char BITs, byte4 xpixel, byte4 ypixel);
byte4 Destroy_Bmp(struct Bmp_s *Bmp);
void	*LoadBmp( char *fname);
void	SaveBmp777(char *fname, struct Image_s *Image);
void	*LoadTif(char *fname);
void	*LoadRAW(char *fname, byte2 numCmpts, byte2 numBitDipth, byte4 width1, byte4 height);
void	*LoadPpm(char *fname);
uchar *MakeBmpStream(struct Image_s *Image);
void SaveRAW(byte4 width, byte4 height, /*byte2 numCmpts,*/ char BitDipth, char *fname, struct Image_s *Image);
void SaveTiff(uchar numCmpts, byte4 width, byte4 xwidth, byte4 height, char BitDipth, char *fname, struct Image_s *Image, uchar PixelFormatData);
void SavePpm(char *fname, struct Image_s *Image, uchar Bitdipth/*, uchar shift_h*/);
void put_ifd_entry(FILE *fp, ubyte2 tag, byte2 type, byte4 count, ubyte4 offset);
byte4 SavePgm(struct Bmp_s *Bmp, char *fname, ubyte2 numCmpts);
void SaveImg(char *fname, struct Image_s *Image );
struct ImageChain_s *ImageChainChildSearch( struct ImageChain_s *ImageC );
struct ImageChain_s *ImageChainParentSearch( struct ImageChain_s *ImageC );
struct ImageChain_s *ImageChainSearch( struct ImageChain_s *ImageC, byte4 numChain);
struct ImageChain_s *ImageChainCreate( struct ImageChain_s *Parent );
struct ImagePlane_s *ImagePlaneCreate( byte4 numBitPlane );
byte4 ImagePlaneDestroy( struct ImagePlane_s *ImagePlane );

struct Image_s* ImageCreate(struct Image_s *image, byte4 width, byte4 height, byte4 tbx0, byte4 tbx1, byte4 tby0, byte4 tby1, char type);
void ImageDestory(struct Image_s *image);
void Print_Image(struct Image_s *image,char *str);
void Print_ImageByte4(struct ImageByte4_s *image, char *str);
void Print_ImageByte2(struct ImageByte2_s *image, char *str);
void Print_ImageChar(struct ImageChar_s *image,char *str);
void	*ImageCharToBit1(struct Image_s *Image);
void	*ImageBit1ToChar(struct Image_s *rImage);
byte4 MatMulti(char *fname1, char *fname2, char *fnameWr);
byte4 floor2(byte4 x, byte4 y);
byte4 RGBtoYUV(struct Image_s *Image, struct Image_s *Stream, uchar ccc, char metric, byte4 offset);
byte4 YUVtoRGB(struct Image_s *RGB, struct Image_s *Y, struct Image_s *U, struct Image_s *V, char metric, byte4 offset);
byte4 DcEnc(struct Image_s *OUT, struct Image_s *IN, byte2 offset, char ccc, char FILTER);
byte4 DcDec(struct Image_s *OUT, struct Image_s *IN, byte4 offset, char foatMode, char ccc);
void ArrangeBigOrder(byte4 *Table, byte4 *Order, byte4 numCode);
void ArrangeLittleOrder(byte4 *Table, byte4 *Order, byte4 numCode);
byte4	Dout4( byte4 *D_, byte4 Addr, byte4 Limit);
byte2	Dout2( byte2 *D_, byte4 Addr, byte4 Limit);
uchar	Dout1( uchar *D_, byte4 Addr, byte4 Limit);
#endif


#endif
