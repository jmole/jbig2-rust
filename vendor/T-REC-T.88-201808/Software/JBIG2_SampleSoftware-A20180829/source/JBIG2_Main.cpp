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
#include	"Jb2Common.h"
#include	"ImageUtil.h"
#include	"Jb2_Debug.h"


int main( int argc, char **argv )
{
	byte4	i;
	byte4	width, height;
	byte4	buf_length;
	FILE	*fp;
	char	*fname1, *fname2, *fname4, *fname3, *fileForm1, *fileForm2, *fname_ini, PageCounter=0, Counter10, Counter00;
	char	encdec;
	struct	StreamChain_s *str;
	struct	Jbig2Parameter_s *Jb2Param;
	struct	Image_s *rImage;
	struct	ImageChain_s *rImagePage, *ImagePage, *ImageTxt, *ImagePat, *ImageHaf, *ImageGen;
	struct	Jb2_ImageChain_s *ImageSym;

	fname_ini = NULL;
	if(argc<9){
		printf("jbig2 -i fname1 -f fname1_extension -o fname2 -F fname2_extension -ini ini_filename\n");
		printf("fname1          : Input fname without extension.\n");
		printf("fname2          : Output fname without extension\n");
		printf("fname1_extension: [jb2] Input extension of jbig2 code stream.\n");
		printf("                  This program carry out for decoder\n");
		printf("                : [bmp] Input extension of binary image data.\n");
		printf("                  This program carry out for encoder\n");
		printf("                : [tif] Input extension of binary image data.\n");
		printf("                  This program carry out for encoder\n");
		printf("fname2_extension: [jb2] Output extension of jbig2 code stream.\n");
		printf("                  This program carry out for encoder\n");
		printf("                : [bmp] Output extension of binary reconstructed image data.\n");
		printf("                  This program carry out for decoder\n");
		printf("                : [tif] Output extension of binary reconstructed image data.\n");
		printf("                  This program carry out for decoder\n");
		printf("ini_filename    : Encode parameter file. If no specified, default parameter is\n");
		printf("                  used. And when decode, this file is disregarded.\n");
		return	EXIT_FAILURE;
	}
	rImage=NULL;
	rImagePage=NULL;
	ImagePage=NULL;
	ImageTxt=NULL;
	ImagePat = NULL;
	ImageHaf = NULL;
	ImageGen = NULL;
	ImageSym = NULL;
	fname1 = new char [256];
	fname2 = new char [256];
	fname3 = new char [256];
	fname4 = new char [256];
	fname_ini= new char [256];
	fileForm1 = new char [8];
	fileForm2 = new char[8];

	i=1;
	while(i<argc){
		if( (!strcmp(argv[i],"-i")) ) {
			strcpy(fname1,argv[i+1]);
			strcpy(fname3,argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-f")) ) {
			strcpy(fileForm1, argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-o")) ) {
			strcpy(fname2,argv[i+1]);
			strcpy(fname4,argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-F")) ) {
			strcpy(fileForm2, argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-ini")) ) {
			strcpy(fname_ini,argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-W")) ) {
			width = atoi(argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-H")) ) {
			height = atoi(argv[i+1]);
			i+=2;
		}
		else{
			printf(" else %d \n ", i);
			i++;
		}
	}

	if( !(strcmp(fileForm1,"bmp") && strcmp(fileForm1,"BMP") && strcmp(fileForm1,"Bmp")) )	encdec=ENC;
	else if( !(strcmp(fileForm1,"tif") && strcmp(fileForm1,"TIF")) )						encdec=ENC;
	else if( !(strcmp(fileForm1,"tiff") ) )													encdec=ENC;
	else if( !(strcmp(fileForm1,"img") ) )													encdec=ENC;
	else if( !(strcmp(fileForm1,"jb2") ) )													encdec=DEC;
	else return	EXIT_FAILURE;

	if(encdec==DEC){
		printf("DEC Start ");
		strcat( fname1, ".jb2" );
		if( NULL==(fp = fopen(fname1, "rb")) ){
			printf("jb2 file open error!\n");
			return	EXIT_FAILURE;
		}
		fseek( fp, 0, SEEK_END );
		buf_length = ftell( fp );
		str = NULL;
		str = StreamChainMake(str, buf_length, NoDiscard);
		fseek( fp, 0, SEEK_SET );
		fread(&str->buf[0], sizeof(char), buf_length, fp);
		fclose(fp);

		if( (Jb2Param = Jb2ParamInit( fname_ini, NULL, DEC )) == (struct	Jbig2Parameter_s *)FALES){
			printf("[main] Jb2ParamInit Error \n");
			return	EXIT_FAILURE;
		}
		if( NULL == (rImagePage=(struct ImageChain_s *)JBIG2_DecMain(str) ) ){
			exit(0);
		}
		if( !(strcmp(fileForm2,"bmp")) ){
			rImagePage = ImageChainParentSearch(rImagePage);
			while(rImagePage!=NULL){
				Counter10   = PageCounter/10;
				Counter00   = PageCounter%10;
				strcpy(fname3, "00");
				fname3[0] = 0x30+Counter10;
				fname3[1] = 0x30+Counter00;
				strcpy(fname2, fname4);
				strcat(fname2, fname3); 
				strcat(fname2,".bmp");
				SaveBmp777( fname2, rImagePage->Image);
				rImagePage = rImagePage->child;
				PageCounter++;
			}
		}
		else if( !(strcmp(fileForm2,"tif")) ){
			rImagePage = ImageChainParentSearch(rImagePage);
			while(rImagePage!=NULL){
				Counter10   = PageCounter/10;
				Counter00   = PageCounter%10;
				strcpy(fname3, "00");
				fname3[0] = 0x30+Counter10;
				fname3[1] = 0x30+Counter00;
				strcpy(fname2, fname4);
				strcat(fname2, fname3); 
				strcat(fname2,".tif");
				SaveTiff(1, (rImagePage->Image->tbx1-rImagePage->Image->tbx0), (rImagePage->Image->tbx1-rImagePage->Image->tbx0), (rImagePage->Image->tby1-rImagePage->Image->tby0), 1, fname2, rImagePage->Image, 0);
				rImagePage = rImagePage->child;
				PageCounter++;
			}
		}
		printf("===>complete\n");
	}
	else{
		printf("ENC Start ");
		ImagePage = ImageChainCreate(ImagePage);
		if( !(strcmp(fileForm1,"tif") && strcmp(fileForm1,"TIF")) ){
			strcat(fname1,".tif");
			if( NULL == (rImage = (Image_s *)LoadTif(fname1)) ){
				printf("input file open error/n");
				return	EXIT_FAILURE;
			}
			ImagePage->Image = (struct Image_s *)Jb2_ImageBit1ToChar(rImage);
		}
		else if( !(strcmp(fileForm1,"bmp") && strcmp(fileForm1,"BMP")) ){
			strcat(fname1,".bmp");
			if( NULL == (rImage=(struct Image_s *)LoadBmp(fname1 )) ){
				printf("input file open error/n");
			}
			ImagePage->Image = (struct Image_s *)Jb2_ImageBit1ToChar(rImage);
		}

		ImagePage = ImageChainSearch(ImagePage, 0);
		str = NULL;
		str = StreamChainMake(str, 0x8000, NoDiscard);
		Jb2Param = Jb2ParamInit( fname_ini, ImagePage, ENC );
		str = JBIG2_EncMain( str, Jb2Param, ImagePage );
		strcat(fname2,".jb2");
		if(FALES==StreamToFile(fname2, str)){
			printf("[main] StreamtoFile Error\n");
			return	EXIT_FAILURE;
		}
		printf("===>complete\n");
	}
	return	EXIT_SUCCESS;
}

