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
//#include	"Jb2_Debug.h"

byte4	Dout4( byte4 *D_, byte4 Addr, byte4 Limit)
{
	if(Addr<0)				return 0;
	else if(Addr>=Limit)	return 0;
	else					return D_[Addr];
}

byte2	Dout2( byte2 *D_, byte4 Addr, byte4 Limit)
{
	if(Addr<0)				return 0;
	else if(Addr>=Limit)	return 0;
	else					return D_[Addr];
}

uchar	Dout1( uchar *D_, byte4 Addr, byte4 Limit)
{
	if(Addr<0)				return 0;
	else if(Addr>=Limit)	return 0;
	else					return D_[Addr];
}



float ClampF(float A, float maxmax, float minmin)
{
	if(A<minmin)	A=minmin;
	if(A>maxmax	)	A=maxmax;
	return	A;
}

struct StreamChain_s *StreamChainMake(struct StreamChain_s *str, byte4 buf_length, char stream_type)
{

	struct StreamChain_s *newS;

	if(buf_length<=0){
		printf("[StreamChainMake::] buf_lengthis zero or negative(%d)\n",buf_length);
		return	NULL;
	}

	if(str==NULL){
		if( NULL == (str = new struct StreamChain_s) ){
			printf("[StreamChainMake] str create error\n");
			return	NULL;
		}

		if( NULL == (str->buf = new uchar [buf_length]) ){
			printf("[StreamChainMake] str->buf create error buf_length=%d\n", buf_length);
			return	NULL;
		}
		str->buf_length=buf_length;
		memset(&str->buf[0], 0, sizeof(uchar)*str->buf_length);
		str->bits=8;
		str->lastbyte=0;
		str->cur_p=0;
		str->child=NULL;
		str->parent=NULL;
		str->numChains=0;
		str->total_p=0;
		str->stream_type = stream_type;
		return str;
	}
	else{
		str=StreamChainChildSearch(str);
		if( NULL == (newS = new struct StreamChain_s) ){
			printf("[StreamChainMake] newS StreamChain_s create error\n");
			return	NULL;
		}
		if( NULL == (newS->buf = new uchar [buf_length]) ){
			printf("[StreamChainMake] newS->buf uchar[%d] create error\n",buf_length);
			return	NULL;
		}

		newS->buf_length=buf_length;
		memset(&newS->buf[0], 0, sizeof(uchar)*buf_length);
		newS->cur_p=0;
		newS->bits=8;
		newS->child=NULL;
		newS->parent=str;
		newS->numChains=str->numChains+1;
		newS->total_p = str->total_p;
		newS->stream_type = stream_type;
		if(str->cur_p!=str->buf_length)
			str->buf_length=str->cur_p;
		str->child = newS;
		return newS;
	}
}

void StreamChainDestory(struct StreamChain_s *str)
{
	while(str->parent != NULL){
		str=str->parent;
	}

	while(str->child != NULL){
		delete [] str->buf;
		str = str->child;
		delete  str->parent;
	}
	delete [] str->buf;
	delete  str;
}

struct StreamChain_s *StreamBitWrite(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length)
{
	while(BitLength>0){
		str->bits--;
		str->buf[str->cur_p] |= ((data>>(BitLength-1))<<str->bits);
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

struct StreamChain_s *StreamBitWriteJ2K(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length)
{
	while(BitLength>0){
		if(str->cur_p==str->buf_length){
			str=StreamChainMake(str, buf_length, str->stream_type);
		}
		else if( (str->cur_p==str->buf_length-1) && (str->bits==0)  ){
			str=StreamChainMake(str, buf_length, str->stream_type);
		}

		if(str->bits==8){
			if((str->stream_type==JPEG2000) && ((str->buf[str->cur_p-1]==0xff) || ((str->cur_p==0)&&(str->lastbyte==0xff))) ){
				if( BitLength >= 7 ){
					str->buf[str->cur_p] = (uchar)( (data>>(BitLength-7)) & 0x7f );
					str->cur_p++;
					str->total_p++;
					str->bits = 8;
					BitLength -= 7;
				}
				else{
					str->bits = 7-BitLength;
					str->buf[str->cur_p] = (uchar)( (data & mask2[ BitLength ]) << str->bits );
					BitLength=0;
					break;
				}
			}
			else{
				if( BitLength > 8 ){
					str->buf[str->cur_p] = (uchar)( (data>>(BitLength-8)) & 0xff );
					str->cur_p++;
					str->total_p++;
					str->bits = 8;
					BitLength -= 8;
				}
				else{
					str->bits = 8 - BitLength ;
					str->buf[str->cur_p] = (uchar)( (data & mask2[ BitLength ]) << str->bits );
					BitLength=0;
					break;
				}
			}
		}
		else{
			if( BitLength >= str->bits ){
				BitLength = BitLength - str->bits;
				str->buf[str->cur_p] = (uchar)(str->buf[str->cur_p] | ( data>>BitLength));
				str->cur_p++;
				str->total_p++;
				str->bits = 8;
			}
			else{
				str->bits -= BitLength;
				str->buf[str->cur_p] = str->buf[str->cur_p] | ( (data & mask2[ BitLength ]) << str->bits );
				BitLength = 0;
				break;
			}
		}
	}
	return str;
}

struct StreamChain_s *StreamBitWriteJPG(struct StreamChain_s *str, ubyte4 data, char BitLength)
{
	char	BitLength__;
	BitLength__ = BitLength;
	BitLength--;

	for(  ; BitLength>=0 ; BitLength--){
		str->bits--;
		str->buf[str->cur_p] |= ((data>>BitLength)<<str->bits);
		if(!str->bits){
			if(str->buf[str->cur_p]!=0xff){
				str->cur_p++;
				str->total_p++;
				str->bits=8;
				if(str->cur_p==str->buf_length){
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
				else if(str->cur_p>str->buf_length){
					printf("[StreamBitWriteJPG] ERROR1 data=%x,BitLength=%d,=>,BitLength=%d cur_p=%x bits=%d\n",data, BitLength__, BitLength, str->cur_p, str->bits);
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
			}
			else{
				str->cur_p++;
				str->total_p++;
				str->bits=8;
				if(str->cur_p==str->buf_length){
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
				else if(str->cur_p>str->buf_length){
					printf("[StreamBitWriteJPG] ERROR2 data=%x,BitLength=%d,=>,BitLength=%d cur_p=%x bits=%d\n",data, BitLength__, BitLength, str->cur_p, str->bits);
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
				str->cur_p++;
				str->total_p++;
				str->bits=8;
				if(str->cur_p==str->buf_length){
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
				else if(str->cur_p>str->buf_length){
					printf("[StreamBitWriteJPG] ERROR3 data=%x,BitLength=%d,=>,BitLength=%d cur_p=%x bits=%d\n",data, BitLength__, BitLength, str->cur_p, str->bits);
					str = StreamChainMake(str, str->buf_length, str->stream_type);
				}
			}
		}
	}
	return str;
}

#if Cplus
struct StreamChain_s *StreamBitWriteJXR(ubyte4 data, byte4 BitLength, struct StreamChain_s *str, byte4 buf_length)
{
	while(BitLength>0){
		str->bits--;
		str->buf[str->cur_p] |= ((data>>(BitLength-1))<<str->bits);
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
#endif

struct StreamChain_s *ByteStuffOutJXR(struct StreamChain_s *str)
{	
	if(str->bits!=8){
		str->cur_p++;
		str->total_p++;
		str->bits=8;
	}
	return	str;
}

struct StreamChain_s *ByteStuffOutJ2K(struct StreamChain_s *str)
{	
	if(str->bits!=8){
		if( (str->bits==0) && (str->buf[str->cur_p]==0xff) ){
			str->bits=8;
			str->cur_p++;
			str->total_p++;
			str->buf[str->cur_p]=0x00;
		}
		str->cur_p++;
		str->total_p++;
		str->bits=8;
	}
	else{
		if(str->buf[str->cur_p-1]==0xff){
			str->cur_p++;
			str->total_p++;
			str->buf[str->cur_p]=0x00;
		}
	}
	return	str;
}

struct StreamChain_s *ByteStuffOutJPG(struct StreamChain_s *str)
{	
	if(str->bits!=8){
		str->buf[str->cur_p] |= (unsigned char)mask2[str->bits];
		str->cur_p++;
		str->total_p++;
		str->bits=8;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}
		if(str->buf[str->cur_p-1] == 0xff){
			str->buf[str->cur_p]=0x00;
			str->cur_p++;
			str->total_p++;
			if(str->cur_p==str->buf_length){
				str = StreamChainMake(str, str->buf_length, str->stream_type);
			}
		}
	}
	else{
		if(str->buf[str->cur_p-1]==0xff){
			str->buf[str->cur_p]=0x00;
			str->cur_p++;
			str->total_p++;
			if(str->cur_p==str->buf_length){
				str = StreamChainMake(str, str->buf_length, str->stream_type);
			}
		}
	}
	return	str;
}

struct StreamChain_s *Stream1ByteWrite(struct StreamChain_s *str, uchar byte, byte4 buf_length )
{

	if(str->bits!=8){
		str->bits=8;
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);
	}


	str->buf[str->cur_p]=byte;
	str->cur_p++;
	str->total_p++;
	if(str->cur_p==str->buf_length)
		str = StreamChainMake(str, str->buf_length, str->stream_type);

	return str;
}

struct StreamChain_s *Stream2ByteWrite(struct StreamChain_s *str, ubyte2 byte, byte4 buf_length, char mode)
{
	char	tempD;

	tempD = BIG_ENDIAN;
	if(str->bits!=8){
		str->bits=8;
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}
	}

	if(mode==tempD){
		//b=s;
		str->buf[str->cur_p]=(uchar)((byte&0xff00)>>8);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);

		str->buf[str->cur_p]=(uchar)(byte&0x00ff);
		str->cur_p++;
		str->total_p++;
		str->bits=8;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);
	}
	else{
		//b=s;
		str->buf[str->cur_p]=(uchar)(byte&0xff);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);

		str->buf[str->cur_p]=(uchar)((byte&0xff00)>>8);
		str->cur_p++;
		str->total_p++;
		str->bits=8;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);
	}

	return str;
}

struct StreamChain_s *Stream4ByteWrite(struct StreamChain_s *str, ubyte4 byte, byte4 buf_length, char mode)
{
	char	tempD;
	tempD = BIG_ENDIAN;

	if(str->bits!=8){
		str->bits=8;
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);
	}

	if( mode==tempD ){
		str->buf[str->cur_p]=(uchar)((byte&0xff000000)>>24);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);

		str->buf[str->cur_p]=(uchar)((byte&0x00ff0000)>>16);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		
		str->buf[str->cur_p]=(uchar)((byte&0x0000ff00)>>8);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length)
			str = StreamChainMake(str, str->buf_length, str->stream_type);

		str->buf[str->cur_p]=(uchar)((byte&0x000000ff));
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}
	}
	else{
		str->buf[str->cur_p]=(uchar)(byte&0xff);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}

		str->buf[str->cur_p]=(uchar)((byte&0x0000ff00)>>8);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}

		str->buf[str->cur_p]=(uchar)((byte&0x00ff0000)>>16);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}

		str->buf[str->cur_p]=(uchar)((byte&0xff000000)>>24);
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}
	}
	return str;
}

struct StreamChain_s *StreamFloatWrite(struct StreamChain_s *str, float byte, byte4 buf_length)
{
	void	*ADDR_byte;
	uchar	*DD;
	if(str->bits!=8){
		str->bits=8;
		str->cur_p++;
		str->total_p++;
		if(str->cur_p==str->buf_length){
			str = StreamChainMake(str, str->buf_length, str->stream_type);
		}
	}

	ADDR_byte = (void*)(&byte);
	DD = (uchar *)ADDR_byte;

	str->buf[str->cur_p]=DD[3];
	str->cur_p++;
	str->total_p++;
	if(str->cur_p==str->buf_length)
		str = StreamChainMake(str, str->buf_length, str->stream_type);

	str->buf[str->cur_p]=DD[2];
	str->cur_p++;
	str->total_p++;
	if(str->cur_p==str->buf_length)
		str = StreamChainMake(str, str->buf_length, str->stream_type);

	str->buf[str->cur_p]=DD[1];
	str->cur_p++;
	str->total_p++;
	if(str->cur_p==str->buf_length)
		str = StreamChainMake(str, str->buf_length, str->stream_type);


	str->buf[str->cur_p]=DD[0];
	str->cur_p++;
	str->total_p++;
	if(str->cur_p==str->buf_length)
		str = StreamChainMake(str, str->buf_length, str->stream_type);

	return str;
}

uchar Stream1ByteRef(StreamChain_s *s, byte4 adr)
{
	byte4	new_adr;
	struct StreamChain_s *rC;

	rC = StreamChainChildSearch(s);

	new_adr = adr;
	while( 1 ){
		if( rC->buf_length<new_adr ){
			new_adr -= rC->buf_length;
			rC=rC->parent;
		}
		else
			break;
	}
	return rC->buf[new_adr];
}

byte2 Stream2ByteRef(StreamChain_s *s, byte4 adr)
{
	byte4 MSB;
	uchar LSB;

	LSB = (byte4)Stream1ByteRef(s, adr);
	MSB = Stream1ByteRef(s, adr+1);
	return  (byte2)((MSB<<8) | LSB);
}

byte4 Stream4ByteRef(StreamChain_s *s, byte4 adr)
{
	byte4 MSB, LSB;

	LSB = (byte4)Stream2ByteRef(s, adr);
	MSB = (byte4)Stream2ByteRef(s, adr+2);
	return  (byte4)((MSB<<16) | LSB);

}

struct StreamChain_s *StreamChainBind(struct StreamChain_s *r, struct StreamChain_s *t)
{
	struct	StreamChain_s *rC, *tP, *tC;

	tP = StreamChainParentSearch(t);
	tC = StreamChainChildSearch(t);

	rC = StreamChainChildSearch(r);

	tP->parent = rC;
	tC->total_p += rC->total_p; 

	rC->child = tP;
	rC->buf_length = rC->cur_p;

	tC = StreamChainChildSearch(tC);

	return	tC;
}

struct StreamChain_s *StreamChainTruncate(struct StreamChain_s *s , byte4 length)
{
	struct	StreamChain_s *s_Trunc, *sC, *sP;
	byte4	length1;
	
	length1 = length;
	sP = StreamChainParentSearch(s);
	sC = StreamChainChildSearch(s);

	s_Trunc=sP;
	length1 -= s_Trunc->cur_p;
	while(length1>0){
		s_Trunc = s_Trunc->child;
		length1 -= s_Trunc->cur_p;
	}

	while(sC!=s_Trunc){
		sC = sC->parent;
		delete sC->child->buf;
		sC->child->buf = NULL;
		delete sC->child;
		sC->child=NULL;
	}

	length1 = 0;
	s=sP;
	while(s->child!=NULL){
		length1 += s->cur_p;
		s = s->child;
	}
	if(length-length1>=0){
		s->cur_p = length-length1;
		s->buf_length = s->cur_p;
		s->total_p = length;
	}

	return s;
}

struct StreamChain_s *StreamChainChildSearch(struct StreamChain_s *r)
{
	struct StreamChain_s *rC;
	rC=r;
	while(rC->child!=NULL){
		rC=rC->child;
	}
	return rC;
}

struct StreamChain_s *StreamChainParentSearch(struct StreamChain_s *r)
{
	struct StreamChain_s *rP;
	rP=r;
	while(rP->parent!=NULL){
		rP=rP->parent;
	}
	return rP;
}

byte4 StreamChainDataLength(struct StreamChain_s *r)
{
	struct StreamChain_s *rC,*rP/*,*rPP*/;
	byte4 length=0;

	rC=StreamChainChildSearch(r);

	rP=StreamChainParentSearch(r);

	while(rP!=rC){
		length+=rP->cur_p;
		//rPP=rP;
		rP=rP->child;
	}
	length+=rC->cur_p;
	return length;
}

byte4 StreamToFile(char *fname, struct StreamChain_s *str)
{
	FILE *fp;


	if( NULL == (fp=fopen(fname,"wb")) ){
		printf("[StreamToFile] %s file open error\n",fname);
		return	FALES;
	}
	while(str->parent!=NULL){
		str=str->parent;
	}

	while(str->child != NULL){
		fwrite(str->buf,sizeof(uchar), str->buf_length, fp);
		str=str->child;
	}
	fwrite(str->buf, sizeof(uchar), str->cur_p, fp);
	fclose(fp);
	
	return TRUE;
}

byte4 StreamChainCounter(struct StreamChain_s *s)
{
	struct StreamChain_s *b;
	byte4 counter=0;
	b=s;
	while(b->parent!=NULL){
		b=b->parent;
	}
	while(b->child != NULL){
		counter+=b->cur_p;
		b=b->child;
	}
	counter+=b->cur_p;
	
	return counter;
}

uchar Ref_1Byte(struct StreamChain_s *s)
{
	if(s->cur_p>=s->buf_length)	return NULL;
	s->cur_p++;
	return s->buf[s->cur_p-1];
}

ubyte2 Ref_2Byte(struct StreamChain_s *s)
{
	if(s->cur_p+1>=s->buf_length)	return NULL;
	s->cur_p+=2;
	return (ubyte2)((s->buf[s->cur_p-2]<<8)|(s->buf[s->cur_p-1]));
}

ubyte2 Ref_2ByteL(struct StreamChain_s *s)
{
	if(s->cur_p+1>=s->buf_length)	return NULL;
	s->cur_p+=2;
	return (ubyte2)((s->buf[s->cur_p-2]) | (s->buf[s->cur_p-1]<<8) );
}

float Ref_Float(struct StreamChain_s *str)
{
	uchar	*DD;
	float	DDD;

	if(str->cur_p+3>=str->buf_length)	
		return NULL;

	DD = (uchar *)&DDD;

	DD[0]=str->buf[str->cur_p+3];
	DD[1]=str->buf[str->cur_p+2];
	DD[2]=str->buf[str->cur_p+1];
	DD[3]=str->buf[str->cur_p+0];
	str->cur_p+=4;

	return	DDD;
}


ubyte4 Ref_4Byte(struct StreamChain_s *s)
{
	if(s->cur_p+3>=s->buf_length)	return NULL;
	s->cur_p+=4;
	return ((s->buf[s->cur_p-4]<<24)|(s->buf[s->cur_p-3]<<16)|(s->buf[s->cur_p-2]<<8)|(s->buf[s->cur_p-1]));
}

ubyte4 Ref_4ByteL(struct StreamChain_s *s)
{
	if(s->cur_p+3>=s->buf_length)	return NULL;
	s->cur_p+=4;
	return ((s->buf[s->cur_p-4]) |(s->buf[s->cur_p-3]<<8)|(s->buf[s->cur_p-2]<<16)|(s->buf[s->cur_p-1]<<24));
}

char Ref_1Bit(struct StreamChain_s *s)
{
	char	data;
	if( s->cur_p >= s->buf_length )	return NULL;

	if(s->buf[s->cur_p-1]==0xff){
		if(s->stream_type==JPEG2000){
			if(s->bits==8){
				s->bits--;
			}
		}
		else if(s->stream_type==JPEG){
			if(s->bits==8){
				s->cur_p++;
			}
		}
	}
	data = s->buf[s->cur_p]&mask[s->bits] ?	1:0;
	s->bits--;
	if(s->bits==0){
		s->bits=8;
		s->cur_p++;
	}
	return	data;

}

byte4 Ref_nBits(struct StreamChain_s *s, uchar bits)
{
	byte4 n=0;
	for(;bits>0;bits--)
		n=(n<<1)+Ref_1Bit(s);
	return n;
}

void Ref_StuffBits(struct StreamChain_s *str, uchar mode)
{
	if(mode==JPEG){
		if(str->bits==8){
			if( (str->buf[str->cur_p] == 0xff) && (str->buf[str->cur_p+1]==0x00) )
				str->cur_p+=2;
		}
		else{
			str->bits=8;
			str->cur_p++;
			if( (str->buf[str->cur_p] == 0xff) && (str->buf[str->cur_p+1]==0x00) )
				str->cur_p+=2;
			else if( (str->buf[str->cur_p-1] == 0xff) && (str->buf[str->cur_p]==0x00) )	
				str->cur_p++;
		}
	}
	else{
		if(str->bits!=8){
			str->bits=8;
			str->cur_p++;
		}
	}
}

byte4 DcEnc(struct Image_s *OUT, struct Image_s *IN, byte2 offset, char ccc, char FILTER)
{
	byte4	i, j,jj;
	byte4	width,height;

	width  = IN->width;
	height = IN->height;

	if(FILTER==1){
		if(IN->type==CHAR){
			byte4	*D_TS, *D_;
			uchar	*S_TS, *S_; 

			S_TS = (uchar *)IN->data;
			D_TS = (byte4 *)OUT->data;
			for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1; j++, jj++){
				S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step+ccc];
				D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
				for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
					*D_ = (byte4)(*S_- offset);
				}
			}
		}
		else if(IN->type==BYTE2){
			byte2	*S_TS, *S_; 
			byte4	*D_TS, *D_;

			S_TS = (byte2 *)IN->data;
			D_TS = (byte4 *)OUT->data;
			for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1; j++, jj++){
				S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step+ccc];
				D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
				for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
					*D_ = (byte4)(*S_- offset);
				}
			}
		}
		else if(IN->type==BYTE4){
			byte4	*S_TS, *S_; 
			byte4	*D_TS, *D_;

			S_TS = (byte4 *)IN->data;
			D_TS = (byte4 *)OUT->data;
			for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1; j++, jj++){
				S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step+ccc];
				D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
				for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
					*D_ = (byte4)(*S_- offset);
				}
			}
		}
	}
	else if(FILTER==0){
		float	*D_TS,*D_;
		if(IN->type==CHAR){
			uchar	*S_TS, *S_; 
			S_TS = (uchar *)IN->data;
			D_TS = (float *)OUT->data;
			for(j=0 ; j<height; j++){
				S_ = &S_TS[j*IN->col1step+ccc];
				D_ = &D_TS[j*OUT->col1step];
				for(i=0 ; i<width ; i++, ++D_, S_+=IN->row1step){
					*D_ = (float)(*S_- offset);
				}
			}
		}
		else if(IN->type==BYTE2){
			byte2	*S_TS, *S_; 
			S_TS = (byte2 *)IN->data;
			D_TS = (float *)OUT->data;
			for(j=0 ; j<height; j++){
				S_ = &S_TS[j*IN->col1step+ccc];
				D_ = &D_TS[j*OUT->col1step];
				for(i=0 ; i<width ; i++, ++D_, S_+=IN->row1step){
					*D_ = (float)(*S_- offset);
				}
			}
		}
		else if(IN->type==BYTE4){
			byte4	*S_TS, *S_; 
			S_TS = (byte4 *)IN->data;
			D_TS = (float *)OUT->data;
			for(j=0 ; j<height; j++){
				S_ = &S_TS[j*IN->col1step+ccc];
				D_ = &D_TS[j*OUT->col1step];
				for(i=0 ; i<width ; i++, ++D_, S_+=IN->row1step){
					*D_ = (float)(*S_- offset);
				}
			}
		}
	}
	return TRUE;
}

byte4 DcDec(struct Image_s *OUT, struct Image_s *IN, byte4 offset, char floatMode, char ccc)
{
	byte4	i, j, jj;
	byte4	width,height;

	width  = OUT->tbx1-OUT->tbx0;
	height = OUT->tby1-OUT->tby0;

	if( (OUT->type==CHAR) && (floatMode==FLOAT4) ){
		uchar	*D_TS, *D_; 
		float	*S_TS, *S_, temp;

		S_TS = (float *)IN->data;
		D_TS = (uchar *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ (float)offset;
				if(temp>255)	temp=255;
				if(temp<0)		temp=0;
				*D_ = (uchar)temp;
			}
		}
	}
	else if( (OUT->type==CHAR) && (floatMode==BYTE4) ){
		uchar	*D_TS, *D_; 
		byte4	*S_TS, *S_, temp;

		S_TS = (byte4 *)IN->data;
		D_TS = (uchar *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ offset;
				if(temp>255)	temp=255;
				if(temp<0)		temp=0;
				*D_ = (uchar)temp;
			}
		}
	}
	else if( (OUT->type==BYTE2) && (floatMode==FLOAT4) ){
		byte2	*D_TS, *D_; 
		float	*S_TS, *S_, temp;

		S_TS = (float *)IN->data;
		D_TS = (byte2 *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ (float)offset;
				if(temp> 32767.0)		temp= 32767.0;
				if(temp<-32767.0)		temp=-32767.0;
				*D_ = (byte2)temp;
			}
		}
	}
	else if( (OUT->type==BYTE2) && (floatMode==BYTE4) ){
		byte2	*D_TS, *D_; 
		byte4	*S_TS, *S_, temp;

		S_TS = (byte4 *)IN->data;
		D_TS = (byte2 *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ offset;
				if(temp> 32767)		temp= 32767;
				if(temp<-32767)		temp=-32767;
				*D_ = (byte2)temp;
			}
		}
	}
	else if( (OUT->type==BYTE4) && (floatMode==FLOAT4) ){
		byte4	*D_TS, *D_; 
		float	*S_TS, *S_, temp;

		S_TS = (float *)IN->data;
		D_TS = (byte4 *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ (float)offset;
				*D_ = (byte4)temp;
			}
		}
	}
	else if( (OUT->type==BYTE4) && (floatMode==BYTE4) ){
		byte4	*D_TS, *D_; 
		byte4	*S_TS, *S_, temp;

		S_TS = (byte4 *)IN->data;
		D_TS = (byte4 *)OUT->data;
		for(j=OUT->tby0, jj=IN->tby0 ; j<OUT->tby1 ; j++, jj++){
			S_ = &S_TS[jj*IN->col1step+IN->tbx0*IN->row1step];
			D_ = &D_TS[j*OUT->col1step+OUT->tbx0*OUT->row1step];
			for(i=OUT->tbx0 ; i<OUT->tbx1 ; i++, D_+=OUT->row1step, S_+=IN->row1step){
				temp = *S_+ offset;
				*D_ = (byte4)temp;
			}
		}
	}
	return TRUE;
}

byte4 YUVtoRGB(struct Image_s *RGB, struct Image_s *Y, struct Image_s *U, struct Image_s *V, char metric, byte4 offset)
{
	byte4	width, height;
	byte4	i,j, ii, jj;
	byte4	col1step, row1step;
	byte4	tempA;
	byte4	tempD;
	byte4	*Y_d, *U_d, *V_d;

	width  = Y->width;
	height = Y->height;
	col1step = Y->col1step;
	row1step = Y->row1step;

	if(RGB->type==CHAR){
		uchar	*D_, *D_TS;
		D_TS = (uchar *)RGB->data;
		if(metric==METRIC_RGB){
			for(j=Y->tby0, jj=0 ; j<Y->tby1 ; j++, jj++){
				Y_d=&Y->data[j*col1step+Y->tbx0];
				U_d=&U->data[j*col1step+U->tbx0];
				V_d=&V->data[j*col1step+V->tbx0];
				D_ = &D_TS[RGB->col1step*jj];
				for( i=Y->tbx0, ii=0 ; i<Y->tbx1 ; i++, ii++, ++Y_d, ++U_d, ++V_d, D_+=RGB->row1step ){
					tempA = *Y_d - ((*U_d + *V_d)>>2);
					tempD = *V_d + tempA +offset;
					if(tempD>255)	tempD=255;
					if(tempD<0)		tempD=0;
					D_[0]=tempD;
					tempD = tempA + offset;
					if(tempD>255)	tempD=255;
					if(tempD<0)		tempD=0;
					D_[1]=tempD;
					tempD = *U_d + tempA +offset;
					if(tempD>255)	tempD=255;
					if(tempD<0)		tempD=0;
					D_[2]=tempD;
				}
			}
		}
		else if(metric==METRIC_BGR){
			for(j=Y->tby0, jj=0 ; j<Y->tby1 ; j++, jj++){
				Y_d=&Y->data[j*col1step+Y->tbx0];
				U_d=&U->data[j*col1step+U->tbx0];
				V_d=&V->data[j*col1step+V->tbx0];
				D_ = &D_TS[RGB->col1step*jj];
				for( i=Y->tbx0, ii=0 ; i<Y->tbx1 ; i++, ii++, ++Y_d, ++U_d, ++V_d, D_+=RGB->row1step ){
					tempA = *Y_d - ((*U_d + *V_d)>>2);
					D_[2] = (uchar)(*V_d + tempA +offset);
					D_[1] = (uchar)(tempA + offset);
					D_[0] = (uchar)(*U_d + tempA +offset);
				}
			}
		}
	}
	if(RGB->type==BYTE2){
		byte2	*D_, *D_TS;
		D_TS = (byte2 *)RGB->data;
		if(metric==METRIC_RGB){
			for(j=Y->tby0, jj=0 ; j<Y->tby1 ; j++, jj++){
				Y_d=&Y->data[j*col1step+Y->tbx0];
				U_d=&U->data[j*col1step+U->tbx0];
				V_d=&V->data[j*col1step+V->tbx0];
				D_ = &D_TS[RGB->col1step*jj];
				for( i=Y->tbx0, ii=0 ; i<Y->tbx1 ; i++, ii++, ++Y_d, ++U_d, ++V_d, D_+=RGB->row1step ){
					tempA = *Y_d - ((*U_d + *V_d)>>2);
					tempD = *V_d + tempA +offset;
					if(tempD>0xffff)	tempD=0xffff;
					if(tempD<0)			tempD=0;
					D_[0]=(ubyte2)tempD;
					tempD = tempA + offset;
					if(tempD>0xffff)	tempD=0xffff;
					if(tempD<0)			tempD=0;
					D_[1]=(ubyte2)tempD;
					tempD = *U_d + tempA +offset;
					if(tempD>0xffff)	tempD=0xffff;
					if(tempD<0)			tempD=0;
					D_[2]=(ubyte2)tempD;
				}
			}
		}
		else if(metric==METRIC_BGR){
			for(j=Y->tby0, jj=0 ; j<Y->tby1 ; j++, jj++){
				Y_d=&Y->data[j*col1step+Y->tbx0];
				U_d=&U->data[j*col1step+U->tbx0];
				V_d=&V->data[j*col1step+V->tbx0];
				D_ = &D_TS[RGB->col1step*jj];
				for( i=Y->tbx0, ii=0 ; i<Y->tbx1 ; i++, ii++, ++Y_d, ++U_d, ++V_d, D_+=RGB->row1step ){
					tempA = *Y_d - ((*U_d + *V_d)>>2);
					D_[2] = (byte2)(*V_d + tempA +offset);
					D_[1] = (byte2)(tempA + offset);
					D_[0] = (byte2)(*U_d + tempA +offset);
				}
			}
		}
	}
	return TRUE;
}

byte4 RGBtoYUV(struct Image_s *Image, struct Image_s *Stream, uchar ccc, char metric, byte4 offset)
{
	byte4	y, x, j;
	byte4	*D;

	if(Stream->type==CHAR){
		uchar	*d_, *d_TS;
		d_TS = (uchar *)Stream->data;
		if(ccc==0){
			offset = offset<<2;
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = d_[0]+(d_[1]<<1)+d_[2]- offset;	//	Y=(R+2*G+B)/4
					*D = (*D)>>2;
				}
			}
		}
		else if( ((ccc==1)&&(metric==METRIC_RGB)) || ((ccc==2)&&(metric==METRIC_BGR)) ){
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[2] - d_[1]);	//	U=B-G(when RGB)	V=R-G(when BGR)
				}
			}
		}
		else {
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[0] - d_[1]);	//	V=R-G(when RGB)	U=B-G(when BGR)
				}
			}
		}
	}
	else if(Stream->type==BYTE2){
		byte2	*d_, *d_TS;
		d_TS = (byte2 *)Stream->data;
		if(ccc==0){
			offset = offset<<2;
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = d_[0]+(d_[1]<<1)+d_[2]- offset;	//	Y=(R+2*G+B)/4
					*D = (*D)>>2;
				}
			}
		}
		else if( ((ccc==1)&&(metric==METRIC_RGB)) || ((ccc==2)&&(metric==METRIC_BGR)) ){
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[2] - d_[1]);	//	U=B-G(when RGB)	V=R-G(when BGR)
				}
			}
		}
		else {
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[0] - d_[1]);	//	V=R-G(when RGB)	U=B-G(when BGR)
				}
			}
		}
	}
	else if(Stream->type==BYTE4){
		byte4	*d_, *d_TS;
		d_TS = (byte4 *)Stream->data;
		if(ccc==0){
			offset = offset<<2;
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = d_[0]+(d_[1]<<1)+d_[2]- offset;	//	Y=(R+2*G+B)/4
					*D = (*D)>>2;
				}
			}
		}
		else if( ((ccc==1)&&(metric==METRIC_RGB)) || ((ccc==2)&&(metric==METRIC_BGR)) ){
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[2] - d_[1]);	//	U=B-G(when RGB)	V=R-G(when BGR)
				}
			}
		}
		else {
			for(y=Image->tby0, j=0 ; y<Image->tby1 ; y++, j++){
				d_= &d_TS[j*Stream->col1step];
				D   = &Image->data[y*Image->col1step+Image->tbx0];
				for(x=Image->tbx0 ;x<Image->tbx1 ; x++, ++D, d_+=Stream->row1step){
					*D = (d_[0] - d_[1]);	//	V=R-G(when RGB)	U=B-G(when BGR)
				}
			}
		}
	}
	return TRUE;
}

byte4 ceil2(byte4 x, byte4 y)
{
	byte4 a;
	if( x%y>0 ){
		a=x/y;
		a++;
	}
	else	a=x/y;
	return a;
}

byte4 FiveOnFourDown(byte4 x, byte4 y)
{
	byte4 a;
	byte4 temp;
	temp = x%y;
	if( temp>0 ){
		if(temp*2>=y){
			a=x/y;
			a++;
		}
		else
			a=x/y;
	}
	else	a=x/y;
	return a;
}

byte4 flor(byte4 x, byte4 y)
{
	byte4 a;
	if( x%y<0 ){
		a=x/y;
		a--;
	}
	else	a=x/y;
	return a;
}

byte4 floor2(byte4 x, byte4 y)
{
	byte4 a;

	if( x%y==0 )	a=x/y;
	else if(x%y<0){	a=x/y; a--;}
	else			a=x/y;
	return a;
}


struct Bmp_s *MakeBmp(char BITs, byte4 xpixel, byte4 ypixel)
{
	struct Bmp_s *Bmp;
	byte4 j;

	Bmp = new struct Bmp_s;
	Bmp->BmpInfo = new struct BmpInfo_s;
#if MEM_DEBUG1
	mem0debug_fp=fopen("mem0debug_fp.txt","a+");
	fprintf(mem0debug_fp,"[MakeBmp] Bmp =%08x size=%d\n", Bmp, sizeof(struct Bmp_s) );
	fprintf(mem0debug_fp,"[MakeBmp] Bmp->BmpInfo =%08x size=%d\n", Bmp->BmpInfo, sizeof(struct BmpInfo_s) );
	fclose(mem0debug_fp);
#endif

	Bmp->BmpInfo->B=0x42;
	Bmp->BmpInfo->M=0x4d;

	if(BITs==1){
		Bmp->BmpInfo->offset = 0x36+8;//+0x400; 
		Bmp->BmpInfo->xwidth = xpixel /8 ; 
		Bmp->BmpInfo->colorbits = 0x0001;
		Bmp->BmpInfo->gamma_length = 8;
		Bmp->BmpInfo->gamma = new uchar [Bmp->BmpInfo->gamma_length];
#if MEM_DEBUG1
	mem0debug_fp=fopen("mem0debug_fp.txt","a+");
	fprintf(mem0debug_fp,"[MakeBmp] Bmp->BmpInfo->gamma =%08x size=%d\n", Bmp->BmpInfo->gamma, sizeof(uchar)*2 );
	fclose(mem0debug_fp);
#endif
	}
	else if(BITs==8){
		Bmp->BmpInfo->offset = 0x36+4*256;
		Bmp->BmpInfo->xwidth = xpixel;
		Bmp->BmpInfo->colorbits = 0x0008;
		Bmp->BmpInfo->gamma_length = 256*4;
		Bmp->BmpInfo->gamma = new uchar [256*4];
		for(j=0;j<256;j++){
			Bmp->BmpInfo->gamma[j*4]   = j;
			Bmp->BmpInfo->gamma[j*4+1] = j;
			Bmp->BmpInfo->gamma[j*4+2] = j;
			Bmp->BmpInfo->gamma[j*4+3] = 0;
		}
#if MEM_DEBUG1
	mem0debug_fp=fopen("mem0debug_fp.txt","a+");
	fprintf(mem0debug_fp,"[MakeBmp] Bmp->BmpInfo->gamma =%08x size=%d\n", Bmp->BmpInfo->gamma, sizeof(uchar)*256 );
	fclose(mem0debug_fp);
#endif
	}
	else if(BITs==24){
		Bmp->BmpInfo->offset = 0x36;
		Bmp->BmpInfo->xwidth = xpixel *3 ;
		Bmp->BmpInfo->colorbits = 0x0018;
		Bmp->BmpInfo->gamma_length = 0;
		Bmp->BmpInfo->gamma = NULL;
	}
	else{
		return NULL;
	}
	if((Bmp->BmpInfo->xwidth%4)!=0) 
		Bmp->BmpInfo->xwidth=((Bmp->BmpInfo->xwidth/4)+1)*4;

	Bmp->BmpInfo->value_of_filesize = Bmp->BmpInfo->xwidth * ypixel + Bmp->BmpInfo->offset;
	Bmp->BmpInfo->dumy1 = 0x00000000;
	Bmp->BmpInfo->infohead = 0x00000028;
	Bmp->BmpInfo->xpixel=xpixel;
	Bmp->BmpInfo->ypixel=ypixel;
	Bmp->BmpInfo->plane=0x0001;
	Bmp->BmpInfo->code_method = 0x00000000;
	Bmp->BmpInfo->size_of_coding = 0x00000000;
	Bmp->BmpInfo->xreso = 0x00000000;
	Bmp->BmpInfo->yreso = 0x00000000;
	Bmp->BmpInfo->colours = 0x00000000;
	Bmp->BmpInfo->import_colour = 0x00000000;

	Bmp->d1 = new uchar [Bmp->BmpInfo->xwidth*Bmp->BmpInfo->ypixel];
	Bmp->d = NULL;

	return Bmp;
}

byte4 Destroy_Bmp(struct Bmp_s *Bmp)
{
	if(Bmp->BmpInfo->gamma_length!=0){
		delete [] Bmp->BmpInfo->gamma;
	}

	delete [] Bmp->d1;
	delete Bmp->BmpInfo;
	delete Bmp;
	
	return TRUE;
}



void *LoadBmp( char *fname )
{
	byte4	Bm, BmSize, tempD, HeaderSize, plane, coding, codingSize, Xreso, Yreso, importColor;
	uchar	gamma[256][3];
	byte4	width, height, xwidth, offset, filesize, n, i, j;
	byte4	row1step, col1step;
	byte2	color_bits;
	uchar	*D_;
	FILE	*fp;
	struct	Image_s *Image=NULL;
	struct	StreamChain_s *str=NULL;

	if( NULL==(fp = fopen(fname, "rb")) ){
		printf("[LoadBmp] bmp file open error!\n");
		return	NULL;
	}
	fseek( fp, 0, SEEK_END );
	filesize = ftell( fp );
	str = StreamChainMake(str, filesize, NoDiscard);
	fseek( fp, 0, SEEK_SET );
	fread(&str->buf[0], sizeof(char), filesize, fp);
	fclose(fp);

	Bm = Ref_2ByteL(str);
	BmSize= Ref_4ByteL(str);
	tempD = Ref_2ByteL(str);
	tempD = Ref_2ByteL(str);
	offset= Ref_4ByteL(str);
	HeaderSize = Ref_4ByteL(str);
	width = Ref_4ByteL(str);	//TempD[21]<<24 | TempD[20]<<16 | TempD[19]<<8 | TempD[18];
	height= Ref_4ByteL(str);	//TempD[25]<<24 | TempD[24]<<16 | TempD[23]<<8 | TempD[22];
	plane = Ref_2ByteL(str);
	color_bits= Ref_2ByteL(str);	//TempD[29]<< 8 | TempD[28]; //
	coding= Ref_4ByteL(str); 
	codingSize= Ref_4ByteL(str); 
	Xreso = Ref_4ByteL(str);
	Yreso = Ref_4ByteL(str);
	tempD = Ref_4ByteL(str);
	importColor = Ref_4ByteL(str);
	if(!importColor)
		importColor=(offset-str->cur_p)/4;

	for(i=0;i<importColor;i++){
		gamma[i][0]=Ref_1Byte(str);
		gamma[i][1]=Ref_1Byte(str);
		gamma[i][2]=Ref_1Byte(str);
		tempD      =Ref_1Byte(str);
	}
	if(offset!=str->cur_p){
		printf("[LoadBmp] fname=%s offset=%d,AferGamma=%d bmp file structure error\n",fname, offset, str->cur_p);
		return	(void *)FALES;
	}
		

	if(color_bits == 1){
		xwidth = ceil2(width, 8);
		xwidth = ceil2(xwidth,4)*4;
		Image = ImageCreate( Image, width, height, 0, width, 0, height, BIT1 );
		if(gamma[0][0]==0xff && gamma[1][0]==0x00)	//0:white 1:black
			Image->MaxValue=-1;
		else
			Image->MaxValue=1;//1:white 0:black
	}
	else if(color_bits == 8){
		Image = ImageCreate( Image, width, height, 0, width, 0, height, CHAR);
	}
	else if(color_bits == 24){
		xwidth=width*3;
		if(xwidth%4!=0)	xwidth=(xwidth/4+1)*4;
		Image = ImageCreate( Image, xwidth, height, 0, width, 0, height, CHAR );
		Image->row1step = 3;
	}
	col1step=Image->col1step;
	row1step=Image->row1step;

	if(color_bits!=1){
		D_ = (uchar *)Image->data;
		for(j=0, n=height-1 ; n>=0 ; j++, n--){
			for( i=0 ; i<xwidth ; i+=Image->row1step){
				D_[j*col1step+i] = str->buf[offset+n*xwidth+i];
			}
		}
	}
	else{
		D_ = (uchar *)Image->data;
		for(j=0, n=height-1 ; n>=0 ; j++, n--){
			memcpy(&D_[j*col1step], &str->buf[offset+n*xwidth], sizeof(uchar)*xwidth);
		}
	}

	StreamChainDestory(str);
	
	return	(void *)Image;
}

void *LoadTif(char *fname)
{
	FILE	*fp;
	byte4	filesize;
	byte4	n, i, j, jj, jjj;
	byte2	tag, numTag, type1;
	byte4	numD, numStrip, numOffset, data, offsetTag;
	byte4	*StripByteCount, *Offset1;
	byte4	StripByteCount_total;
	byte2	*BitDipth;
	uchar	*BitDipth_byte;
	byte2	BitDipth_total;
	byte4	StripHeight=0;
	byte4	numCmpts, ccc;
	byte4	minV=0;
	byte4	maxV=0;
	byte4	width1,height1;
	byte4	xwidth1;
	byte4	MaxValue;
	char	metric=2;
	char	NumBytePerPixel;
	char	color=1;
	char	compress;
	struct	Image_s * Image;
	uchar	*image;

	fp = fopen(fname, "rb");
	fseek( fp, 0, SEEK_END );
	filesize = ftell( fp );
	image = new uchar [filesize];
	fseek( fp, 0, SEEK_SET );
	fread(&image[0], sizeof(uchar), filesize, fp);
	fclose(fp);

	//TAG analize
	offsetTag = (image[7]<<24) + (image[6]<<16) +  (image[5]<<8) + image[4];
	numTag = (image[offsetTag+1]<<8) + image[offsetTag];
	offsetTag +=2;
	for (n=0; n<numTag; n++) {
		tag  = (image[n*12+offsetTag+ 1]<< 8) + image[n*12+offsetTag+ 0];
		type1= (image[n*12+offsetTag+ 3]<< 8) + image[n*12+offsetTag+ 2];
		numD = (image[n*12+offsetTag+ 7]<<24) + (image[n*12+offsetTag+ 6]<<16) + (image[n*12+offsetTag+ 5]<<8) + image[n*12+offsetTag+ 4];
		data = (image[n*12+offsetTag+11]<<24) + (image[n*12+offsetTag+10]<<16) + (image[n*12+offsetTag+ 9]<<8) + image[n*12+offsetTag+ 8];
		switch (tag){
			case TagImageWidth :				//100
				width1 = data;
				break;
			case TagImageHeight :				//101
				height1 = data;
				break;
			case TagBitsPerSample :				//102
				numCmpts=numD;
				BitDipth_byte = new uchar [numCmpts];
				BitDipth = new byte2 [numCmpts];
				if(numCmpts==1){
					BitDipth[0] = data;
					BitDipth_total=data;
					BitDipth_byte[0] = 1;
					MaxValue = 0x0ff;
					if(data>8){
						BitDipth_byte[0]=2;
						MaxValue = 0x0ffff;
					}
					if(data>16){
						BitDipth_byte[0]=4;
						MaxValue = 0x7fffffff;
					}
				}
				else{
					BitDipth_total=0;
					MaxValue=0x0ff;
					for( ccc=0 ; ccc<numCmpts ; ccc++ ){
						BitDipth[ccc] = (image[data+1]<<8)+image[data];
						BitDipth_byte[ccc] = 1;
						if(BitDipth[ccc]>8){
							BitDipth_byte[ccc]=2;
							MaxValue=0x0ffff;
						}
						if(BitDipth[ccc]>16){
							BitDipth_byte[ccc]=4;
							MaxValue = 0x7fffffff;
						}
						data+=2;
						BitDipth_total += BitDipth[ccc];
					}
				}
				break;
			case TagPhotometric :
				metric = (char)data;
				break;
			case TagCompression :
				compress=data;
				break;
			case TagSamplesPerPixel :			//115
				color = (char)data;
				break;
			case TagRowsPerStrip :				//116
				StripHeight = data;
				break;
			case TagMinValue :
				minV= data;
				break;
			case TagMaxValue :
				maxV=data;
				break;
			case TagXResolution :
				break;
			case TagYResolution :
				break;
			case TagProgressive :
				break;
			case TagResolutionUnit :
				break;
			case TagStripOffsets :			//111
				numOffset = numD;
				if(numD==1){
					Offset1 = new byte4 [1];
					Offset1[0] = data;
				}
				else{
					Offset1 = new byte4 [numD];
					for(i=0;i<numD;i++){
						Offset1[i] = (image[data+i*4+3]<<24) + (image[data+i*4+2]<<16) + (image[data+i*4+1]<<8) + image[data+i*4];
					}
				}
				break;
			case TagStripByteCounts :		//117
				numStrip = numD;
				if(numStrip==1){
					StripByteCount = new byte4 [1];
					StripByteCount[0] = data;
					StripByteCount_total = data;
				}
				else{
					StripByteCount = new byte4 [numStrip];
					StripByteCount_total = 0;
					for(i=0;i<numStrip;i++){
						StripByteCount[i] = (image[data+i*4+3]<<24) + (image[data+i*4+2]<<16) + (image[data+i*4+1]<<8) + image[data+i*4];
						StripByteCount_total += StripByteCount[i];
					}
				}
				break;
			default:
				break;
		}
	}
	if(numStrip !=numOffset ){
		printf("[LoadTif] TIF file format error. numStrip=%d numOffset=%d\n",numStrip, numOffset);
		return	NULL;
	}

	NumBytePerPixel = 1;
	for(ccc=0;ccc<numCmpts;ccc++){
		if( (BitDipth[ccc]>8) && (NumBytePerPixel<2) )	NumBytePerPixel = 2;
		else if(BitDipth[ccc]>16)						NumBytePerPixel = 4;
	}
	
	xwidth1 = width1 * color;
	if(BitDipth[0]==1){
		uchar	*ImageD;
		Image = ImageCreate(NULL, xwidth1, height1, 0, width1, 0, height1, BIT1);
		ImageD = (uchar *)Image->data;
		jjj=0;
		for(jj=0 ; jj<numStrip-1 ; jj++){
			for(j=0 ; j<StripHeight ; j++, jjj++){
				memcpy(&ImageD[Image->col1step*jjj], &image[ Offset1[jj] + j*ceil2(xwidth1,8) ], sizeof(uchar)*ceil2(xwidth1,8) );
			}
		}
		for( j=0; jjj<height1 ; j++, jjj++){
			memcpy(&ImageD[Image->col1step*jjj], &image[ Offset1[jj] + j*ceil2(xwidth1,8) ], sizeof(uchar)*ceil2(xwidth1,8) );
		}
		j=j;
	}
	else if(NumBytePerPixel==1){
		uchar	*ImageD;
		Image = ImageCreate(NULL, xwidth1, height1, 0, width1, 0, height1, CHAR);
		ImageD = (uchar *)Image->data;
		jjj=0;
		for(jj=0 ; jj<numStrip-1 ; jj++){
			for(j=0 ; j<StripHeight ; j++, jjj++){
				memcpy(&ImageD[Image->col1step*jjj], &image[ Offset1[jj] + j*xwidth1 ], sizeof(uchar)*xwidth1);
			}
		}
		for( j=0; jjj<height1 ; j++, jjj++){
			memcpy(&ImageD[Image->col1step*jjj], &image[ Offset1[jj] + j*xwidth1 ], sizeof(uchar)*xwidth1);
		}
	}
	else if(NumBytePerPixel==2){
		ubyte2	*ImageD;
		Image = ImageCreate(NULL, xwidth1, height1, 0, width1, 0, height1, BYTE2);
		ImageD = (ubyte2 *)Image->data;
		for(jj=0;jj<numStrip;jj++){
			for(j=0 ; j<StripHeight ; j++){
				memcpy(ImageD, &image[ Offset1[jj] + j*xwidth1*2 ], sizeof(ubyte2)*xwidth1);
				ImageD = &ImageD[Image->col1step];
			}
		}
	}
	else if(NumBytePerPixel==4){
		byte4	*ImageD;
		Image = ImageCreate(NULL, xwidth1, height1, 0, width1, 0, height1, BYTE4);
		ImageD = (byte4 *)Image->data;
		for(jj=0;jj<numStrip;jj++){
			for(j=0 ; j<StripHeight ; j++){
				memcpy(ImageD, &image[ Offset1[jj] + j*xwidth1*4 ], sizeof(byte4)*xwidth1);
				ImageD = &ImageD[Image->col1step];
			}
		}
	}
	if(maxV==0)
		Image->MaxValue = MaxValue;
	else
		Image->MaxValue = maxV;
	Image->row1step = color;
	Image->width = width1;

	delete	[]	Offset1;
	delete	[]	StripByteCount;
	return	(void *)Image;
	
}


void *LoadRAW(char *fname, byte2 numCmpts, byte2 numBitDipth, byte4 width1, byte4 height1)
{
	byte4	filesize;
	FILE	*fp;
	byte4	xwidth0, xwidth, i, j;
	INSTAN	offsetAddr;
	byte4	maxValue, bits;
	uchar	*tempD;
	struct	Image_s *Image;
	uchar	*ImageD1, *ImageD1_;
	byte2	*ImageD2, *ImageD2_, *ImageD2_TS;
	byte4	*ImageD4, *ImageD4_;

	if(NULL == (fp = fopen(fname, "rb")) ){
		printf("[LoadRAW] %s file open error\n",fname);
		return	NULL;
	}
	fseek( fp, 0, SEEK_END );
	filesize = ftell( fp );
	tempD = new uchar [filesize];
	fseek( fp, 0, SEEK_SET);
	fread(&tempD[0], sizeof(uchar), filesize, fp);
	fclose(fp);
	
	xwidth0 = numCmpts*width1;
	if( (xwidth0*height1*numBitDipth) != filesize){
		printf("[LoadRAW] width1=%d height1=%d numCmpts=%d numBitDipth=%d filesize=%d\n",width1,height1,numCmpts,numBitDipth,filesize);
		return	NULL;
	}

	Image = new	struct Image_s;
	if(numBitDipth==1){
		if(xwidth0%16)	xwidth = (xwidth0/16+1)*16;
		else			xwidth = xwidth0;
		Image->numData = xwidth * height1+15;

		ImageD1_ = new uchar [Image->numData];
		memset(&ImageD1_[0],0,sizeof(uchar)*Image->numData );
		offsetAddr = (INSTAN)ImageD1_ %16;
		ImageD1 = &ImageD1_[offsetAddr];
		for(j=0;j<height1;j++)
			memcpy(&ImageD1[j*xwidth], &tempD[j*xwidth0], sizeof(uchar)*xwidth0);
		Image->data = (byte4 *)ImageD1;
		Image->Pdata = (byte4 *)ImageD1_;
		Image->type = CHAR;
		bits = 255;
	}
	else if(numBitDipth==2){
		if(xwidth0%8)	xwidth = (xwidth0/8+1)*8;
		else			xwidth = xwidth0;
		ImageD2_ = new byte2 [xwidth * height1+7];
		memset(&ImageD2_[0],0,sizeof(byte2)*(xwidth * height1+ 7) );
		offsetAddr = (INSTAN)ImageD2_ %16;
		ImageD2 = &ImageD2_[offsetAddr/2];
		for(j=0;j<height1;j++)
			memcpy(&ImageD2[j*xwidth], &tempD[j*xwidth0*2], sizeof(byte2)*xwidth0);
		Image->data = (byte4 *)ImageD2;
		Image->Pdata = (byte4 *)ImageD2_;
		Image->numData = xwidth * height1+ 7;
		Image->type = BYTE2;
		maxValue=0;
		ImageD2_TS=(byte2 *)Image->data;
		for(j=0;j<height1;j++, ImageD2_TS +=Image->col1step){
			ImageD2 = ImageD2_TS;
			for(i=0;i<xwidth0;i++, ++ImageD2){
				if( maxValue>abs(ImageD2[i]) )
					maxValue=abs(ImageD2[i]);
			}
		}
		if(0xff<maxValue && maxValue<=0xfff )
			bits=0xfff;
		else if(0xfff<maxValue && maxValue<=0xffff )
			bits=0xffff;

	}
	else if(numBitDipth==4){
		if(xwidth0%16)	xwidth = (xwidth0/4+1)*4;
		else			xwidth = xwidth0;
		ImageD4_ = new byte4 [xwidth * height1+3];
		memset(&ImageD4_[0],0,sizeof(byte4)*(xwidth * height1+ 3) );
		offsetAddr = (INSTAN)ImageD4_ %16;
		ImageD4 = &ImageD4_[offsetAddr/4];
		for(j=0;j<height1;j++)
			memcpy(&ImageD4[j*xwidth], &tempD[j*xwidth0*4], sizeof(byte4)*xwidth0);
		Image->data = (byte4 *)ImageD4;
		Image->Pdata = (byte4 *)ImageD4_;
		Image->numData = xwidth * height1+ 3;
		Image->type = BYTE4;
		maxValue=0;
		bits=0x7fffffff;
	}
	Image->tbx0 = 0;
	Image->tbx1 = width1;
	Image->tby0 = 0;
	Image->tby1 = height1;
	Image->width  = width1;
	Image->height = height1;
	Image->col1step = xwidth;
	Image->row1step = numCmpts;
	Image->MaxValue=bits+1;

	delete	tempD;
	return	(void *)Image;
}

void *LoadPpm(char *fname)
{
	byte4	/*fh,*/ filesize;
	FILE	*fp;
	byte4	width, height;
	byte4	xwidth, xheight;
	byte4	bits;
	byte4	i,j,ii;
	float	tempF;
	uchar	*tempD;
	char	minusF;
	struct	Image_s *Image;

	if(NULL == (fp = fopen(fname, "rb")) ){
		printf("[LoadPpm] %s file open error\n",fname);
		return	NULL;
	}
	fseek( fp, 0, SEEK_END );
	filesize = ftell( fp );
	tempD = new uchar [filesize];
	fseek( fp, 0, SEEK_SET);
	fread(&tempD[0], sizeof(uchar), filesize, fp);
	fclose(fp);

	//P5
	if( (tempD[0]==0x50) && (tempD[1]==0x35)  && (tempD[2]==0x0a || tempD[2]==0x20) ){
		i=3;
		if(tempD[i]==0x23){	//#
			while(tempD[i] != 0x0a)		i++;
			i++;
		}
		width=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			width *=10;
			width += (tempD[i]-0x30);
			i++;
		}
		i++;
		height=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			height *=10;
			height += (tempD[i]-0x30);
			i++;
		}
		i++;
		bits=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			bits *=10;
			bits += (tempD[i]-0x30);
			i++;
		}
		i++;

		if(bits<0x100){
			uchar	*D_TS;
			Image = ImageCreate(NULL, width, height, 0, width, 0, height, CHAR);
			D_TS = (uchar *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				memcpy(&D_TS[0], &tempD[i], width*sizeof(uchar));
				i+=width;
			}
		}
		else if(bits<0x10000){
			ubyte2	*D_TS, *D_;
			Image = ImageCreate(NULL, width, height, 0, width, 0, height, BYTE2);
			D_TS = (ubyte2 *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				for(ii=0, D_=D_TS ; ii<width ; ii++, D_=&D_[1]){
					D_[0]=(ubyte2)(tempD[i]*0x100)+(byte4)tempD[i+1];
					i+=2;
				}
			}
		}
		Image->row1step=1;
	}
	//P6
	else if( (tempD[0]==0x50) && (tempD[1]==0x36)  && (tempD[2]==0x0a || tempD[2]==0x20) ){
		i=3;
		if(tempD[i]==0x23){	//#
			while(tempD[i] != 0x0a)	i++;
			i++;
		}
		width=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			width *=10;
			width += (tempD[i]-0x30);
			i++;
		}
		xwidth = ((width+15)/16)*16;
		i++;
		height=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			height *=10;
			height += (tempD[i]-0x30);
			i++;
		}
		xheight = ((height+15)/16)*16;
		i++;
		bits=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			bits *=10;
			bits += (tempD[i]-0x30);
			i++;
		}
		i++;
		if(bits<0x100){
			uchar	*D_TS;
			Image = ImageCreate(NULL, xwidth*3, xheight, 0, width, 0, height, CHAR);
			D_TS = (uchar *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				memcpy(&D_TS[0], &tempD[i], width*3*sizeof(uchar));
				i+=(width*3);
			}
		}
		else if(bits<0x10000){
			byte4	ii;
			ubyte2	*D_TS, *D_;
			Image = ImageCreate(NULL, xwidth*3, xheight, 0, width, 0, height, BYTE2);
			D_TS = (ubyte2 *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				D_ = D_TS;
				for(ii=0 ; ii<width ; ii++, D_ = &D_[3]){
					D_[0] = (ubyte2)( (ubyte2)tempD[i+0]*0x100 + (ubyte2)tempD[i+1] );
					D_[1] = (ubyte2)( (ubyte2)tempD[i+2]*0x100 + (ubyte2)tempD[i+3] );
					D_[2] = (ubyte2)( (ubyte2)tempD[i+4]*0x100 + (ubyte2)tempD[i+5] );
					i += 6;
				}
			}
		}
		Image->row1step=3;
	}
	//Pf	Pf is Gray
	else if( (tempD[0]==0x50) && (tempD[1]==0x66)  && (tempD[2]==0x0a || tempD[2]==0x20) ){
		i=3;
		if(tempD[i]==0x23){	//#
			while(tempD[i] != 0x0a)	i++;
			i++;
		}
		width=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			width *=10;
			width += (tempD[i]-0x30);
			i++;
		}
		xwidth = ((width+15)/16)*16;
		i++;
		height=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			height *=10;
			height += (tempD[i]-0x30);
			i++;
		}
		xheight = ((height+15)/16)*16;
		i++;
		if(tempD[i]==0x2d){	minusF=-1;	i++;}
		else				minusF=1;
		bits=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			bits *=10;
			bits += (tempD[i]-0x30);
			i++;
		}
		bits*=minusF;

		i++;
		if(bits<0){
			float	*D_TS;
			Image = ImageCreate(NULL, xwidth, xheight, 0, width, 0, height, FLOAT4);
			D_TS = (float *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				memcpy(&D_TS[0], &tempD[i], width*sizeof(float));
				i+=(width*4);
			}
		}
		else{
			byte4	ii;
			float	*D_TS, *D_;
			Image = ImageCreate(NULL, xwidth, xheight, 0, width, 0, height, FLOAT4);
			D_TS = (float *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				D_ = D_TS;
				for(ii=0 ; ii<width ; ii++, D_ = &D_[1]){
					tempF = (float)( (tempD[i+3]<<0x18) | (tempD[i+2]<<0x10) | (tempD[i+1]<<0x8) | (tempD[i]) );
					D_[0] = tempF;
					i += 4;
				}
			}
		}
		Image->row1step=1;
	}
	//PF	PF is Color
	else if( (tempD[0]==0x50) && (tempD[1]==0x46)  && (tempD[2]==0x0a || tempD[2]==0x20) ){
		i=3;
		if(tempD[i]==0x23){	//#
			while(tempD[i] != 0x0a)	i++;
			i++;
		}
		width=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			width *=10;
			width += (tempD[i]-0x30);
			i++;
		}
		xwidth = ((width+15)/16)*16;
		i++;
		height=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			height *=10;
			height += (tempD[i]-0x30);
			i++;
		}
		xheight = ((height+15)/16)*16;
		i++;
		if(tempD[i]==0x2d){	minusF=-1;	i++;}
		else				minusF=1;
		bits=0;
		while( !(tempD[i]==0x0a || tempD[i]==0x20) ){
			bits *=10;
			bits += (tempD[i]-0x30);
			i++;
		}
		bits*=minusF;

		i++;
		if(bits<0){
			float	*D_TS;
			Image = ImageCreate(NULL, xwidth*3, xheight, 0, width, 0, height, FLOAT4);
			D_TS = (float *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				memcpy(&D_TS[0], &tempD[i], width*3*sizeof(float));
				i+=(width*4*3);
			}
		}
		else{
			byte4	ii;
			float	*D_TS, *D_;
			Image = ImageCreate(NULL, xwidth*3, xheight, 0, width, 0, height, FLOAT4);
			D_TS = (float *)Image->data;
			for(j=0;j<height;j++, D_TS+=Image->col1step){
				D_ = D_TS;
				for(ii=0 ; ii<width ; ii++, D_ = &D_[3]){
					tempF = (float)( (tempD[i+3]<<0x18) | (tempD[i+2]<<0x10) | (tempD[i+1]<<0x8) | (tempD[i]) );
					D_[0] = tempF;
					i += 4;
					tempF = (float)( (tempD[i+3]<<0x18) | (tempD[i+2]<<0x10) | (tempD[i+1]<<0x8) | (tempD[i]) );
					D_[1] = tempF;
					i += 4;
					tempF = (float)( (tempD[i+3]<<0x18) | (tempD[i+2]<<0x10) | (tempD[i+1]<<0x8) | (tempD[i]) );
					D_[2] = tempF;
					i += 4;
				}
			}
		}
		Image->row1step=3;
	}
	Image->width=width;
	Image->tbx0=0;
	Image->tbx1=width;
	Image->tby0=0;
	Image->tby1=height;
	Image->MaxValue=bits;
	delete	[] tempD;
	return	(void *)Image;

}

uchar *MakeBmpStream( struct Image_s *Image )
{
	byte4	filesize, offsetD, header=0x28, n, xwidth;
	byte4	i,j,width, height;
	uchar	*tempD, *tempD_TS;
	uchar	*bmpinfoDD;

	width  = Image->tbx1-Image->tbx0;
	height = Image->height;
	xwidth = width*Image->row1step;
	if(xwidth%4)	xwidth = (xwidth/4+1)*4;
	filesize = height * xwidth;

	if( (Image->type==CHAR) && (Image->row1step==1) ){
		filesize += 0436;
		offsetD = 0x436;
		bmpinfoDD = new uchar [filesize];
		tempD_TS = (uchar *)Image->data;
		for( j=0 ; j<height ; j++ ){
			tempD = &tempD_TS[(height-1-j)*Image->col1step];
			for(i=0;i<width;i++){
				bmpinfoDD[offsetD+j*xwidth+i] =tempD[i];
			}
		}
		for(n=0; n<256;n++){
			bmpinfoDD[4*n+0x36]=n;	bmpinfoDD[4*n+0x37]=n;	bmpinfoDD[4*n+0x38]=n;	bmpinfoDD[4*n+0x39]=0;
		}
	}
	else if( (Image->type==CHAR) && (Image->row1step==3) ){
		filesize +=0x36;
		offsetD = 0x36;
		bmpinfoDD = new uchar [filesize];
		tempD_TS = (uchar *)Image->data;
		for( j=0 ; j<height ; j++ ){
			tempD = &tempD_TS[(height-1-j)*Image->col1step];
			for(i=0;i<width;i++){
				for(n=0;n<3;n++){
					bmpinfoDD[offsetD+j*xwidth+i*Image->row1step+n] =tempD[i*3+(2-n)];
				}
			}
		}
	}
	else if( (Image->type==BIT1) && (Image->row1step==1) ){
		filesize +=0x3e;
		offsetD = 0x3e;
		bmpinfoDD = new uchar [offsetD];
		bmpinfoDD[0x36]=255;	bmpinfoDD[0x37]=255;	bmpinfoDD[0x38]=255;	bmpinfoDD[0x39]=0;
		bmpinfoDD[0x3a]=0;		bmpinfoDD[0x3b]=0;		bmpinfoDD[0x3c]=0;		bmpinfoDD[0x3d]=0;
	}
	else{
		printf("[MakeBmpStream] File format of Image_s *Image is not much to file format of bmp.\n");
		return	NULL;
	}

	bmpinfoDD[0] = 0x42;		//putc('B',fp);				//0		B
	bmpinfoDD[1] = 0x4d;		//putc('M',fp);				//		M
	bmpinfoDD[2] = filesize&0xff;	bmpinfoDD[3] = (filesize>>8)&0xff;	bmpinfoDD[4] = (filesize>>16)&0xff;		bmpinfoDD[5] = (filesize>>24)&0xff;
	for(n=6;n<10;n++)	bmpinfoDD[n]=0;
	bmpinfoDD[0xa] = offsetD&0xff;	bmpinfoDD[0xb] = (offsetD>>8)&0xff;	bmpinfoDD[0xc] = (offsetD>>16)&0xff;	bmpinfoDD[0xd] = (offsetD>>24)&0xff;	//offset
	bmpinfoDD[0xe] = header&0xff;	bmpinfoDD[0xf] = (header>>8)&0xff;	bmpinfoDD[0x10] = (header>>16)&0xff;	bmpinfoDD[0x11] = (header>>24)&0xff;	//header
	bmpinfoDD[0x12] = width&0xff;	bmpinfoDD[0x13] = (width>>8)&0xff;	bmpinfoDD[0x14] = (width>>16)&0xff;		bmpinfoDD[0x15] = (width>>24)&0xff;		//width
	bmpinfoDD[0x16] = height&0xff;	bmpinfoDD[0x17] = (height>>8)&0xff;	bmpinfoDD[0x18] = (height>>16)&0xff;	bmpinfoDD[0x19] = (height>>24)&0xff;	//height
	bmpinfoDD[0x1a] = 1;			bmpinfoDD[0x1b] = 0;																								//plane
	bmpinfoDD[0x1c] = Image->row1step*8;	bmpinfoDD[0x1d] = 0;																						//color
	for(n=0x1e;n<0x36;n++)	bmpinfoDD[n]=0;	

//	fh = _open(fname, (_O_WRONLY|_O_CREAT|_O_BINARY), _S_IWRITE );
//	n=_write(fh, bmpinfoDD, sizeof(uchar)*filesize);
//	close(fh);
	return	bmpinfoDD;
}

void SaveBmp777( char *fname, struct Image_s *Image )
{
	FILE	*fp;
	byte4	n, i;
	byte4	filesize, offset, xpixel, xwidth, height, plane, numCmpts, resoX=0x2e23, resoY=0x2e23, dumy=0, header=40, siro=0x000000, kuro=0xffffff;
	uchar	*D_TS, *D_;

	if(Image->type==BIT1){
		xpixel = Image->tbx1-Image->tbx0;
		if(xpixel%8)	xwidth = xpixel/8+1;
		else			xwidth = xpixel/8;
		if(xwidth%4)	xwidth = (xwidth/4+1)*4;
		height = Image->tby1-Image->tby0;
		plane=1;
		numCmpts = 1;
		offset = 54+4*2;
		filesize = xwidth*height+offset;
	}
	else if(Image->type==CHAR){
		xpixel = Image->tbx1-Image->tbx0;
		if(Image->row1step==1){
			if(xpixel%4)	xwidth = (xpixel/4+1)*4;
			else			xwidth = xpixel;
			height = Image->tby1-Image->tby0;
			plane=1;
			numCmpts = 8;
			offset = 54+4*256;
			filesize = xwidth*height+256*4+offset;
		}
		else if(Image->row1step==3){
			if((xpixel*3)%4)	xwidth = ((xpixel*3)/4+1)*4;
			else				xwidth = xpixel*3;
			height = Image->tby1-Image->tby0;
			plane=1;
			numCmpts = 24;
			offset = 54;
			filesize = xwidth*height+offset;
		}
	}
	else{
	}

	fp=fopen(fname,"wb");
	putc('B',fp);				//0		B
	putc('M',fp);				//		M
	putw(filesize, fp);			//2		filesize
	putw(dumy, fp);				//6		dumy
	putw(offset, fp);			//10	offset
	putw(header, fp);			//14	header
	putw(xpixel, fp);			//18	X
	putw(height, fp);			//22	Y
	putc(plane&0xff, fp);		//26	plane
	putc(0x00, fp);
	putc(numCmpts&0xff, fp);	//28	
	putc(0x00, fp);
	putw(dumy, fp);				//30	compress	
	putw(dumy, fp);				//34	num size
	putw(resoX, fp);			//38	resoX
	putw(resoY, fp);			//42	resoY
	putw(dumy, fp);				//46
	putw(dumy, fp);				//50
	if(Image->type==BIT1){
		if(Image->MaxValue==-1){
			putw(kuro,fp);	putw(siro,fp);
		}
		else{
			putw(siro,fp);	putw(kuro,fp);
		}
		D_TS = (uchar *)Image->data;
		for( n=Image->tby1-1 ; n>=Image->tby0 ; n-- ){
			fwrite(&D_TS[n*Image->col1step], sizeof(uchar),xwidth,fp);
		}
	}
	else if(Image->type==CHAR){
		if(numCmpts==8){
			for(n=0;n<256;n++){
				putc(n, fp);	putc(n,fp);	putc(n,fp);	putc(0,fp);
			}
		}
		D_TS = (uchar *)Image->data;
		for(n=Image->tby1-1, D_TS = &D_TS[n*Image->col1step]  ; n>=Image->tby0 ; n--, D_TS=&D_TS[n*Image->col1step] ){
			D_ = D_TS;
			for(i=0 ; i<xpixel ; i++, D_=&D_[3]){
				putc(D_[2], fp);
				putc(D_[1], fp);
				putc(D_[0], fp);
			}
			for(i=xpixel*3 ; i<xwidth ; i++)
				putc(0, fp);
			D_TS = (uchar *)Image->data;
		}
	}
	else{
		printf("[SaveBmp777] struct Image_s error\n");
		fclose(fp);
	}
	fclose(fp);
	
}

void put_ifd_entry(FILE *fp, ubyte2 tag, byte2 type, byte4 count, ubyte4 offset)
{	
	put2(tag,fp);
	put2(type,fp);
	put4(count,fp);
	if (type==3 && count==1) {
		put2((byte2)offset, fp);
		put2(0, fp); 
	}
	else
		put4(offset, fp); 
}

void SaveTiff(uchar numCmpts, byte4 width, byte4 xwidth, byte4 height, char BitDipth, char *fname, struct Image_s *Image, uchar PixelFormatData)
{
	FILE	*fh;
	byte4	i, j;
	char	metric=2;
	char	photometric;
	byte4	bitsize;
	byte4	bitoffset, dataoffset, formatoffset=8;
	byte4	*D4;
	byte2	*D2;
	uchar	*D1, *D1_TS;
	byte4	PixFormat1;
	uchar	*PixFormat;

	PixFormat = new	uchar [16];
	PixFormat1 = PixFmt1;
	PixFormat[0x0]=(PixFormat1>>24)&0xff;	PixFormat[0x1]=(PixFormat1>>16)&0xff;	PixFormat[0x2]=(PixFormat1>> 8)&0xff;	PixFormat[0x3]=PixFormat1&0xff;
	PixFormat1 = PixFmt2;
	PixFormat[0x4]=(PixFormat1>>24)&0xff;	PixFormat[0x5]=(PixFormat1>>16)&0xff;	PixFormat[0x6]=(PixFormat1>> 8)&0xff;	PixFormat[0x7]=PixFormat1&0xff;
	PixFormat1 = PixFmt3;
	PixFormat[0x8]=(PixFormat1>>24)&0xff;	PixFormat[0x9]=(PixFormat1>>16)&0xff;	PixFormat[0xa]=(PixFormat1>> 8)&0xff;	PixFormat[0xb]=PixFormat1&0xff;
	PixFormat1 = PixFmt4;
	PixFormat[0xc]=(PixFormat1>>16)&0xff;	PixFormat[0xd]=(PixFormat1>> 8)&0xff;	PixFormat[0xe]=PixFormat1&0xff;			PixFormat[0xf]=PixelFormatData;

	fh = fopen( fname, "wb");

	char magic = 'I';
	int nentry = 10;
	if (numCmpts == 4) /* CMYK */
		nentry += 1;
	
	bitsize = numCmpts>=3 ? 2*numCmpts : 0;
	bitoffset = 0x18 + 2 + 12*nentry + 4;
	dataoffset = bitoffset + bitsize;

	if(numCmpts==3)			photometric = 2;
	else if(numCmpts==4)	photometric = 5;
	else					photometric = 1;

	putc(magic, fh);
	putc(magic, fh);
	put2(0x002A,fh);
	put4(0x18, fh);
	for(i=0;i<16;i++)
		putc(PixFormat[i],fh);
	put2(nentry, fh);
	put_ifd_entry(fh, TagPixelFormat,     1, 16, formatoffset);
	put_ifd_entry(fh, TagImageWidth,      4, 1, width);
	put_ifd_entry(fh, TagImageHeight,     4, 1, height);
	put_ifd_entry(fh, TagBitsPerSample,   3, numCmpts, (numCmpts>1 ? bitoffset:BitDipth));
	put_ifd_entry(fh, TagCompression,     3, 1, 1);
	put_ifd_entry(fh, TagPhotometric,     3, 1, photometric);
	put_ifd_entry(fh, TagStripOffsets,    4, 1, dataoffset);
	put_ifd_entry(fh, TagSamplesPerPixel, 3, 1, numCmpts);
	put_ifd_entry(fh, TagRowsPerStrip,    4, 1, height);
	put_ifd_entry(fh, TagStripByteCounts, 4, 1, width * height * numCmpts * ((BitDipth+7)/8));
	if (numCmpts == 4)
		put_ifd_entry(fh, TagInkSet,    3, 1, 1);

	put4(0, fh);
	if(PixelFormatData==0x09){
		put2(0x0005,fh);put2(0x0005,fh);put2(0x0005,fh);
	}
	else if(PixelFormatData==0x0A){
		put2(0x0005,fh);put2(0x0006,fh);put2(0x0005,fh);
	}
	else if(PixelFormatData==0x13){
		put2(0x000A,fh);put2(0x000A,fh);put2(0x000A,fh);
	}
	else if (numCmpts>=3) {
		for (i=0; i<numCmpts; i++){
			put2(BitDipth, fh);
		}
	}

	if(BitDipth==1){
		uchar	tempD;
		D1_TS = (uchar *)Image->data;
		xwidth = ceil2(width,8);
		if(Image->MaxValue!=-1){
			for( j=0 ; j<height ; j++ ){
				fwrite(&D1_TS[j*Image->col1step], sizeof(uchar), xwidth, fh);
			}
		}
		else{
			for( j=0 ; j<height ; j++ ){
				for(i=0;i<xwidth;i++){
					tempD = D1_TS[j*Image->col1step+i];
					tempD = ~tempD;
					fwrite(&tempD, sizeof(uchar), 1, fh);
				}
			}
		}
	}
	else if(BitDipth<=8){
		D1 = (uchar *)Image->data;
		for( i=0 ; i<height ; i++ ){
			fwrite(&D1[i*Image->col1step], sizeof(uchar), xwidth, fh);
		}
	}
	else if(BitDipth<=16){
		D2 = (byte2 *)Image->data;
		for( i=0; i<height ; i++ ){
			fwrite(&D2[i*Image->col1step], sizeof(byte2), xwidth, fh);
		}
	}
	else if(BitDipth<=32){
		D4 = (byte4 *)Image->data;
		for( i=0; i<height ; i++ ){
			fwrite(&D4[i*xwidth], sizeof(byte4), xwidth, fh);
		}
	}
	fclose(fh);
	
}

void SaveRAW(byte4 xwidth, byte4 height, char BitDipth, char *fname, struct Image_s *Image)
{
	FILE	*fh;
	byte4	j;

	fh = fopen( fname, "wb");

	if(BitDipth<=8){
		uchar	*D1, *D1_TS;
		D1_TS = (uchar *)Image->data;
		for(j=0 ; j<height ; j++, D1_TS+=Image->col1step){
			D1 = D1_TS;
			fwrite(D1, sizeof(uchar), xwidth, fh);
		}
	}
	else if(BitDipth<=16){
		ubyte2	*D1, *D1_TS;
		D1_TS = (ubyte2 *)Image->data;
		for(j=0 ; j<height ; j++, D1_TS+=Image->col1step){
			D1 = D1_TS;
			fwrite(D1, sizeof(ubyte2), xwidth, fh);
		}
	}
	else if(BitDipth<=32){
		byte4	*D1, *D1_TS;
		D1_TS = (byte4 *)Image->data;
		for(j=0 ; j<height ; j++, D1_TS+=Image->col1step){
			D1 = D1_TS;
			fwrite(D1, sizeof(byte4), xwidth, fh);
		}
	}
	fclose(fh);
	
}


void SavePpm(char *fname, struct Image_s *Image, uchar Bitdipth/*, uchar shift_h*/)
{
	FILE	*fh;
	char	*buf;
	byte4	i,j;
	uchar	floatF;
	uchar	bits;


	if(Image->MaxValue==-1){
		bits=Bitdipth;
	}
	else{
		if(Image->MaxValue<0x100)			bits=8;
		else if(Image->MaxValue<0x200)		bits=9;
		else if(Image->MaxValue<0x400)		bits=10;
		else if(Image->MaxValue<0x800)		bits=11;
		else if(Image->MaxValue<0x1000)		bits=12;
		else if(Image->MaxValue<0x2000)		bits=13;
		else if(Image->MaxValue<0x4000)		bits=14;
		else if(Image->MaxValue<0x8000)		bits=15;
		else if(Image->MaxValue<0x10000)	bits=16;
		else								bits=32;
	}
	fh = fopen( fname, "wb");

	buf = new char [32];

	if( (Image->type==FLOAT4) || (Image->type==FLOAT2) ){
		floatF=1;
		if(Image->row1step==1)			strcpy(buf,"Pf\n");
		else if(Image->row1step==3)		strcpy(buf,"PF\n");
		fwrite(buf, sizeof(char), strlen(buf), fh);
		//width height bitDipth
		fprintf(fh,"%d\n%d\n",(Image->tbx1-Image->tbx0), (Image->tby1-Image->tby0));
		fprintf(fh,"%d\n", -1*((1<<bits)-1));	//LITTLE Endian
	}
	else{
		floatF=0;
		if(Image->row1step==1)			strcpy(buf,"P5\n");
		else if(Image->row1step==3)		strcpy(buf,"P6\n");
		fwrite(buf, sizeof(char), strlen(buf), fh);
		//width height bitDipth
		fprintf(fh,"%d\n%d\n",(Image->tbx1-Image->tbx0), (Image->tby1-Image->tby0));
		fprintf(fh,"%d\n", ((1<<bits)-1));
	}


	if(floatF){
		float *D_TS;
		D_TS = (float *)Image->data;
		for(j=0;j<Image->height;j++, D_TS+=Image->col1step ){
			fwrite(D_TS, sizeof(float), (Image->tbx1-Image->tbx0)*Image->row1step, fh);
		}
	}
	else{
		if(bits<=8){
			uchar	*D_TS;
			D_TS = (uchar *)Image->data;
			for( j=0 ; j<Image->height ; j++, D_TS+=Image->col1step){
				fwrite(D_TS, sizeof(uchar), (Image->tbx1-Image->tbx0)*Image->row1step, fh);
			}
		}
		else if(bits<=16){
			ubyte2	*D_TS, *D_;
			D_TS = (ubyte2 *)Image->data;
			for( j=Image->tby0; j<Image->tby1 ; j++, D_TS+=Image->col1step ){
				D_=D_TS;
				for( i=0 ; i<(Image->tbx1-Image->tbx0)*Image->row1step ; i++, ++D_ ){
					putc( (uchar)(*D_>>8), fh);
					putc( (uchar)(*D_&0xff), fh);
				}
			}
		}
	}

	delete [] buf;
}

void SaveImg(char *fname, struct Image_s *Image )
{
	FILE	*fp;
	byte4	jjj, width;
	uchar	*D_TS, *D_;

	width = ceil2((Image->tbx1-Image->tbx0),8);
	fp = fopen( fname, "wb");
	D_TS = (uchar *)Image->data;
	for( jjj=0 ; jjj<Image->height ; jjj++, D_TS+=Image->col1step){
		D_ = D_TS;
		fwrite(D_, sizeof(uchar), width, fp);
	}
	fclose(fp);
}

float Ref_ASCII_Froat(struct StreamChain_s *s, uchar *DFlag)
{
	byte4	i, lastI;
	byte4	DataE, DDDD2, Sign2;
	float	DataD, DDDD0, Sign0;

	i=0;
	DFlag[i] = Ref_1Byte(s);
	while( (DD_BracketR != DFlag[i]) && (DD_Space != DFlag[i]) ){
		i++;
		DFlag[i] = Ref_1Byte(s);
	}

	if( DFlag[0]==DD_Minus ){
		Sign0=(float)-1.0;
		i=1;
	}
	else{
		Sign0 = (float)1.0;
		i=0;
	}

	lastI=0;
	DDDD0=(float)0.1;
	while(DFlag[i]!=DD_Point){
		i++;
		lastI++;
		DDDD0 *= (float)10.0;
	}

	i -= lastI;
	DataD = (float)0.0;
	while(lastI){
		DataD += ((DFlag[i]-0x30)*DDDD0);
		DDDD0 /= (float)10.0;
		i++;
		lastI--;
	}

	i++;
	DDDD0 = (float)0.1;
	while(DFlag[i]!=DD_e){
		DataD += ((DFlag[i]-0x30)*DDDD0);
		DDDD0 *= (float)0.1;
		i++;
	}
	DataD *= Sign0;

	i++;
	if(DFlag[i]==DD_Minus)
		Sign2=1;
	else
		Sign2=0;
	i++;

	lastI=0;
	DDDD2 = 1;
	while( (DFlag[i]!=DD_Space) && (DFlag[i]!=DD_BracketR) ){
		i++;
		lastI++;
		DDDD2 *= 10;
	}
	DDDD2 /=10;
	i -= lastI;
	DataE = 0;
	while(lastI){
		DataE += ( (DFlag[i]-0x30)*DDDD2 );
		DDDD2 /=10;
		i++;
		lastI--;
	}
	DDDD0 = (float)1.0;
	if(Sign2)
		for( ; DataE>0 ; DataE-- )
			DDDD0 *= (float)0.1;
	else{
		for( ; DataE>0 ; DataE-- )
			DDDD0 *= (float)10.0;
	}
	DataD *= DDDD0;
	return	DataD;
}

struct StreamChain_s *Write_ASCII_Froat(struct StreamChain_s *s, float DataD)
{
	byte4	i, eI, DDD0;
	float	DataE;
	uchar	D100, D10, D1;

	if(DataD<0){
		s = Stream1ByteWrite(s, DD_Minus, s->buf_length);
		DataE=DataD*-1;
	}
	else
		DataE=DataD;

	DDD0 = (byte4)DataE;
	if(DataE==0){
		eI=0;
	}
	else{
		if(DDD0){
			eI=0;
			while(DataE>=(float)10.0){
				DataE /= (float)10.0;
				eI++;
			}
		}
		else{
			eI=0;
			while(DataE<(float)1.0){
				DataE *= (float)10;
				eI--;
			}
		}
	}

	DDD0 = (byte4)DataE;
	s = Stream1ByteWrite(s, (DDD0+0x30), s->buf_length);

	s = Stream1ByteWrite(s, DD_Point, s->buf_length);

	DataE = DataE - (float)DDD0;
	for( i=0 ; i<6 ; i++ ){
		DataE *= (float)10.0;
		DDD0 = (byte4)DataE;
		s = Stream1ByteWrite( s, (DDD0+0x30), s->buf_length);
		DataE -= (float)DDD0;
	}

	s = Stream1ByteWrite( s, DD_e, s->buf_length);

	if(eI<0)
		s = Stream1ByteWrite( s, DD_Minus, s->buf_length);
	else
		s = Stream1ByteWrite( s, DD_Plus, s->buf_length);

	eI = abs(eI);
	D100 = (uchar)(eI/100);
	D10  = (uchar)((eI-(byte4)D100)/10);
	D1   = (uchar)(eI-(byte4)D100-(byte4)D10);
	s = Stream1ByteWrite( s, (D100+0x30), s->buf_length);
	s = Stream1ByteWrite( s, (D10+0x30),  s->buf_length);
	s = Stream1ByteWrite( s, (D1+0x30),   s->buf_length);
	return	s;
}

ubyte2 get2(FILE*fp){
	ubyte2 a,b;
	a=getc(fp);
	b=getc(fp);
	return (ubyte2)((b<<8)+a);
}

void put2(byte2 d, FILE*fp){
	uchar a,b;
	a=(uchar)(d>>8);
	b=(uchar)(d&0xff);
	putc(b,fp);
	putc(a,fp);

}

void put4(byte4 e, FILE*fp){
	uchar a,b,c,d;
	a=(uchar)((e>>24)&0xff);
	b=(uchar)((e>>16)&0xff);
	c=(uchar)((e>> 8)&0xff);
	d=(uchar)(e&0xff);
	putc(d,fp);
	putc(c,fp);
	putc(b,fp);
	putc(a,fp);

}

byte4 floorlog2(ubyte4 x)
{
	byte4 y;
	if(x<0)	return NULL;
	y = 0;
	while (x > 1) {
		x >>= 1;
		++y;
	}
	return y;
}

byte4 ceil2log2(ubyte4 x)
{
	byte4 y;
	if(x<0)	return FALES;
	y=0;
	while (x > 1) {
		x >>= 1;
		++y;
	}
	return y+1;
}


byte4 fRounding(double fIn)
{
	byte4 O;
	if(fIn>=0){
		O=(byte4)fIn;
		if( (fIn-(float)O)>=0.5 )
			O++;
	}
	else{
		O=(byte4)fIn;
		if( ((float)O-fIn)>=0.5 )
			O--;
	}
	return	O;
}

byte4 Rounding( byte4 In, byte4 y)
{
	float	fD;
	byte4	D;

	fD = (float)In /  (float)y;
	D  = In / y;
	if(fD>=0.0){
		if( (fD-(float)D) >=0.5 )
			return	D+1;
		else
			return	D;
	}
	else{
		if( ((float)D-fD) >=0.5 )
			return	D-1;
		else
			return	D;
	}
}

byte4 Umod(byte4 Data, ubyte4 Modulo)
{
	if(Data<0)
		Data = Data&(Modulo-1);
	else
		Data = Data%Modulo;
	return	Data;
}


struct ImageChain_s *ImageChainSearch( struct ImageChain_s *ImageC, byte4 numChain)
{
	if(ImageC->numChains==numChain)
		return	ImageC;

	ImageC = ImageChainParentSearch( ImageC );

	while(ImageC->numChains!=numChain){
		ImageC=ImageC->child;
	}
	return	ImageC;
}

struct ImageChain_s *ImageChainParentSearch( struct ImageChain_s *ImageC )
{
	while(ImageC->parent !=NULL ){
		ImageC=ImageC->parent;
	}
	return	ImageC;
}

struct ImageChain_s *ImageChainChildSearch( struct ImageChain_s *ImageC )
{
	while(ImageC->child !=NULL ){
		ImageC=ImageC->child;
	}
	return	ImageC;
}

struct ImageChain_s *ImageChainCreate( struct ImageChain_s *Parent)
{
	struct	ImageChain_s *ImageC;
	if(Parent!=NULL){
		Parent = ImageChainChildSearch(Parent);
		if( NULL == (ImageC = new struct ImageChain_s) ){
			printf("[ImageChainCreate] ImageChain_s create error\n");
			return	NULL;
		}
		ImageC->Image=NULL;
		ImageC->parent = Parent;
		ImageC->child=NULL;
		ImageC->numChains = Parent->numChains+1;
		Parent->child = ImageC;
	}
	else{
		if( NULL == (ImageC = new struct ImageChain_s) ){
			printf("[ImageChainCreate] ImageChain_s create error\n");
			return	NULL;
		}
		ImageC->Image=NULL;
		ImageC->parent=NULL;
		ImageC->child=NULL;
		ImageC->numChains=0;
	}
	return ImageC;
}

struct ImagePlane_s *ImagePlaneCreate( byte4 numBitPlane )
{
	struct	ImagePlane_s *ImageP;
	
	ImageP = new struct ImagePlane_s;
	ImageP->numBitPlane=numBitPlane;
	ImageP->Image = new struct Image_s *[numBitPlane];
	return ImageP;
}

byte4 ImagePlaneDestroy( struct ImagePlane_s *ImagePlane )
{
	byte4	kkk;
	
	for( kkk=0 ; kkk<ImagePlane->numBitPlane; kkk++){
		delete	[] ImagePlane->Image[kkk]->Pdata;
		delete	ImagePlane->Image[kkk];
	}
	delete	ImagePlane;

	return TRUE;
}

struct Image_s* ImageCreate(struct Image_s *image, byte4 width, byte4 height, byte4 tbx0, byte4 tbx1, byte4 tby0, byte4 tby1, char type)
{
	byte4	numData;
	INSTAN	offsetAddr;
	byte4	xwidth;
	if(image==NULL){
		if( NULL == (image = new struct Image_s) ){
			printf("[ImageCreate] ImageChain_s create error\n");
			return	NULL;
		}
		image->MaxValue=-1;
	}

	if(type==BIT1){
		uchar	*dataP0;
		byte4	width1;

		width1 = ceil2(width,8);
		if(width1%16)	xwidth=(width1/16+1)*16;
		else			xwidth = width1;
		numData = xwidth*height+15;
		if( NULL == (dataP0 = new uchar [numData]) ){
			printf("[ImageCreate] dataP0 uchar[%d]memory create error\n",numData);
			return	NULL;
		}
		image->Pdata = (byte4 *)dataP0;
		offsetAddr = (INSTAN)(&dataP0[0]);
		offsetAddr = offsetAddr%16;
		image->data = (byte4 *)(&dataP0[offsetAddr]);
		memset(image->Pdata, 0, sizeof(uchar)*numData);
		image->type=BIT1;
	}
	else if(type==CHAR){
		uchar	*dataP0;
		if(width%16)	xwidth=(width/16+1)*16;
		else			xwidth = width;
		numData = xwidth*height+15;
		dataP0 = new uchar [numData];
		if(dataP0 == NULL){
			printf("[ImageCreate::] dataP0=0x%x uchar[%d]memory create error \n",dataP0, numData);
			return	NULL;
		}
		image->Pdata = (byte4 *)dataP0;
		offsetAddr = (INSTAN)(&dataP0[0]);
		offsetAddr = offsetAddr%16;
		image->data = (byte4 *)(&dataP0[offsetAddr]);
		memset(image->Pdata, 0, sizeof(uchar)*numData);
		image->type=CHAR;
	}
	else if(type==BYTE2){
		byte2	*dataP1;

		if(width%8)		xwidth=(width/8+1)*8;
		else			xwidth=width;
		numData = xwidth*height+7;
		if( NULL == (dataP1 = new byte2 [numData]) ){
			printf("[ImageCreate::] dataP1 byte2[%d]memory create error\n",numData);
			return	NULL;
		}
		image->Pdata = (byte4 *)dataP1;
		offsetAddr = (INSTAN)(&dataP1[0]);

		offsetAddr = offsetAddr%16;
		offsetAddr /= 2;
		image->data = (byte4 *)(&dataP1[offsetAddr]);
		memset(image->Pdata, 0, sizeof(byte2)*numData);
		image->type=BYTE2;
	}
	else if(type==BYTE4){
		byte4	*dataP2;

		if(width%4)		xwidth=(width/4+1)*4;
		else			xwidth = width;
		numData = xwidth*height+3;
		if( NULL == (dataP2 = new byte4 [numData]) ){
			printf("[ImageCreate::] dataP2 byte4[%d]memory create error\n",numData);
			return	NULL;
		}
		image->Pdata = dataP2;
		offsetAddr = (INSTAN)(&dataP2[0]);
		offsetAddr = offsetAddr%16;
		offsetAddr = offsetAddr/4;
		image->data = &dataP2[offsetAddr];
		memset(image->Pdata, 0, sizeof(byte4)*numData);
		image->type=BYTE4;
	}
	else if(type==FLOAT4){
		float	*dataP3;
		if(width%4)		xwidth=(width/4+1)*4;
		else			xwidth = width;
		numData = xwidth*height+3;
		if( NULL == (dataP3 = new float [numData]) ){
			printf("[ImageCreate::] dataP3 byte4[%d]memory create error\n",numData);
			return	NULL;
		}
		image->Pdata = (byte4 *)dataP3;
		offsetAddr = (INSTAN)(&dataP3[0]);
		offsetAddr = offsetAddr%16;
		offsetAddr = offsetAddr/4;
		image->data = (byte4 *)&dataP3[offsetAddr];
		memset(image->Pdata, 0, sizeof(float)*numData);
		image->type=FLOAT4;
	}
	image->col1step=xwidth;
	image->row1step=1;
	image->numData = numData;
	image->tbx0=tbx0;
	image->tbx1=tbx1;
	image->tby0=tby0;
	image->tby1=tby1;
	image->height = height;
	image->width =  image->tbx1-image->tbx0;
	return image;
}


void ImageDestory(struct Image_s *image)
{
	delete [] image->Pdata;
	delete image;
}

void Print_Image(struct Image_s *image, char *str)
{
	byte4 i0,i1,j0,j1;
	byte4 i,j,width,height;

	i0=image->tbx0;
	i1=image->tbx1;
	j0=image->tby0;
	j1=image->tby1;
	width=image->width;
	height = image->height;
	printf("%s\n",str);
	printf("tbx0=%d tbx1=%d tby0=%d tby1=%d\n",i0,i1,j0,j1);

	if(image->type==BYTE4){
		byte4 temp;
		for(j=0;j<height;j++){
			for(i=0;i<width;i++){
				temp = image->data[j*image->col1step+i];
				printf("%d,",temp);
			}
			printf("\n");
		}
	}
	if(image->type==BYTE2){
		byte2 temp;
		byte2	*D;
		D = (byte2 *)image->data;
		for(j=0;j<height;j++){
			for(i=0;i<width;i++){
				temp = D[j*image->col1step+i];
				printf("%d,",temp);
			}
			printf("\n");
		}
	}
	if(image->type==CHAR){
		uchar temp;
		uchar	*D;
		D = (uchar *)image->data;
		for(j=0;j<height;j++){
			for(i=0;i<width;i++){
				temp = D[j*image->col1step+i];
				printf("%d,",temp);
			}
			printf("\n");
		}
	}
	printf("\n");
}

byte4 MatMulti(char *fname1, char *fname2, char *fnameWr)
{
	byte4 i,j,ii,jj,width1,width2,height1,height2;
	FILE *fpr1,*fpr2,*fpw;
	long double *fpr1DATA,*fpr2DATA,temp,CCC;
	long double total=0.0;

	fpr1=fopen(fname1,"r");
	fscanf(fpr1,"%d,%d\n",&width1,&height1);
	fpr2=fopen(fname2,"r");
	fscanf(fpr2,"%d,%d\n",&width2,&height2);

	if(width1==height2){
	}
	else{
		return FALES;
	}
	fclose(fpr2);

	fpr1DATA = new long double [width1];
	fpr2DATA = new long double [width1];//height2 is equal to width1.
	fpw=fopen(fnameWr,"w");
	for(j=0;j<height1;j++){
		for(ii=0;ii<width1;ii++){
			fscanf(fpr1,"%lf,",&temp);
			fpr1DATA[ii]=temp;
		}
		for(i=0;i<width2;i++){
			fpr2=fopen(fname2,"r");
			fscanf(fpr2,"%d,%d\n",&width2,&height2);
			for(jj=0;jj<height2;jj++){
				for(ii=0;ii<i;ii++){
					fscanf(fpr2,"%lf,",&temp);
				}
				fscanf(fpr2,"%lf,",&temp);
				fpr2DATA[jj]=temp;
				for(ii=i+1;ii<width2;ii++){
					fscanf(fpr2,"%lf,",&temp);
				}
			}
			fclose(fpr2);

			CCC=0.0;
			for(jj=0;jj<width1;jj++){
				CCC+=fpr1DATA[jj]*fpr2DATA[jj];
			}
			total+=CCC;
			fprintf(fpw,"%lf,",CCC);
		}
		fprintf(fpw,"\n");
	}
	fclose(fpw);
	fclose(fpr1);
	delete [] fpr1DATA;
	delete [] fpr2DATA;
	printf("total=,%lf\n",total);
	printf("complete!!!\n");
		
	return TRUE;
}

void ArrangeBigOrder(byte4 *Table, byte4 *Order, byte4 numCode)
{
	byte4	maxV, lll;
	byte4	iii,jjj,kkk;
	byte4	*tempOrder;
	bool	flag;

	tempOrder = new byte4 [numCode];
	memcpy(tempOrder, Order, sizeof(byte4)*numCode);

	for(iii=0 ; iii<numCode ; iii++){
		maxV=-1;
		for(jjj=0 ; jjj<numCode ; jjj++, flag=1){
			for(kkk=0;kkk<iii;kkk++){
				if(Order[kkk]==tempOrder[jjj]){
					kkk=iii;
					flag=0;
				}	
			}
			if(flag){
				if(maxV<Table[tempOrder[jjj]]){
					maxV=Table[tempOrder[jjj]];
					lll=tempOrder[jjj];
				}
			}
		}
		Order[iii]=lll;
	}
}
void ArrangeLittleOrder(byte4 *Table, byte4 *Order, byte4 numCode)
{
	byte4	minV, lll;
	byte4	iii,jjj,kkk;
	byte4	*tempOrder;
	bool	flag;

	tempOrder = new byte4 [numCode];
	memcpy(tempOrder, Order, sizeof(byte4)*numCode);

	for(iii=0 ; iii<numCode ; iii++){
		minV=0x7fffffff;
		for(jjj=0 ; jjj<numCode ; jjj++, flag=1){
			for(kkk=0;kkk<iii;kkk++){
				if(Order[kkk]==tempOrder[jjj]){
					kkk=iii;
					flag=0;
				}	
			}
			if(flag){
				if(minV>Table[tempOrder[jjj]]){
					minV=Table[tempOrder[jjj]];
					lll=tempOrder[jjj];
				}
			}
		}
		Order[iii]=lll;
	}
}

void	*ImageBit1ToChar(struct Image_s *rImage)
{
	struct	Image_s *Image=NULL;
	byte4	width, height, s_col1step, t_col1step;
	byte4	i,j,k;
	uchar	*Ds_TS, *Dt_TS, *Ds_, *Dt_;
	uchar	tempD;

	width =rImage->tbx1-rImage->tbx0;
	height=rImage->tby1-rImage->tby0;
	Image = ImageCreate( Image, width, height, 0, width, 0,height, CHAR);
	s_col1step = rImage->col1step;
	t_col1step = Image->col1step;

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
	Image->MaxValue=rImage->MaxValue;
	return	(void *)Image;
}

void	*ImageCharToBit1(struct Image_s *Image)
{
	struct	Image_s *rImage=NULL;
	byte4	iii,jjj,kkk;
	byte4	width, height, col1step, r_col1step;
	uchar	tempD;
	uchar	*D_TS, *D_, *Dr_TS, *Dr_;

	if(Image->type!=CHAR)
		return	NULL;

	width = (Image->tbx1-Image->tbx0);
	height = (Image->tby1-Image->tby0);
	rImage = ImageCreate( rImage, width, height, 0, width, 0, height, BIT1);
	r_col1step = rImage->col1step;
	col1step = Image->col1step;
	Dr_TS = (uchar *)rImage->data;
	D_TS = (uchar *)Image->data;

	for(jjj=0; jjj<height ; jjj++, D_TS+=col1step, Dr_TS+=r_col1step){
		D_ = D_TS;
		Dr_ = Dr_TS;
		for( tempD=0, iii=0, kkk=7 ; iii<width ; iii++, kkk-- ){
			tempD |= (D_[iii]<<kkk);
			if(kkk==0){
				*Dr_ = tempD;
				kkk=8;
				tempD=0;
				++Dr_;
			}
		}
		if(kkk!=7)
			*Dr_=tempD;
	}
	rImage->MaxValue=Image->MaxValue;
	return	(void *)rImage;
}
