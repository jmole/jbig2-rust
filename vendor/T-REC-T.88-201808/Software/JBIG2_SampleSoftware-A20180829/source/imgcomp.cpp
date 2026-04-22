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



/******************************************************************************
* Includes.
******************************************************************************/

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#include <float.h>
#include <assert.h>
#include "imgcomp.h"



/******************************************************************************\
* Main program.
\******************************************************************************/

byte4 main(int argc, char **argv)
{
	char	fname1[256],fname2[256], fname3[256], form1[256], form2[256];
	char	METRIC1[8];
	char	LOG[8];
	char	i,flag;
	struct	Image_s *Image1=NULL, *Image2=NULL;
	float	d;
	FILE	*fp;
	byte4	len;
	byte4	*addr;
	byte4	*diff;
	byte4	width1,height1,width2, height2;
	byte2	numCmpts1,numCmpts2;
	byte2	numBitDipth1,numBitDipth2;
	char	HDphoto1=0;
	char	HDphoto2=0;

	if(argc<=2){
		printf("argment error!!\n");
		usage();
		exit(0);
	}

	i=1;
	flag=0;
	while(i<argc){
		if( !strcmp(argv[i],"-T") ){
			strcpy(fname2,argv[i+1]);
			i+=2;
		}
		else if( !strcmp(argv[i],"-F") ){
			strcpy(form2,argv[i+1]);
			if( (!strcmp(form2,"raw")) || (!strcmp(form2,"RAW")) ){
				width2 = atoi(argv[i+2]);
				height2=atoi(argv[i+3]);
				numCmpts2=atoi(argv[i+4]);
				numBitDipth2=atoi(argv[i+5]);
				if(!strcmp(argv[i+6],"-HDphoto"))	HDphoto2=1;
				i+=4;
			}
			else
				i+=2;
		}
		else if( !strcmp(argv[i],"-t") ){
			strcpy(fname1,argv[i+1]);
			i+=2;
		}
		else if( !strcmp(argv[i],"-f") ){
			strcpy(form1,argv[i+1]);
			if( (!strcmp(form1,"raw")) || (!strcmp(form1,"RAW")) ){
				width1 = atoi(argv[i+2]);
				height1=atoi(argv[i+3]);
				numCmpts1=atoi(argv[i+4]);
				numBitDipth1=atoi(argv[i+5]);
				i+=4;
			}
			else
				i+=2;
		}
		else if( (!strcmp(argv[i],"-m")) || (!strcmp(argv[i],"-M")) ) {
			strcpy(METRIC1,argv[i+1]);
			i+=2;
		}
		else if( (!strcmp(argv[i],"-log")) || (!strcmp(argv[i],"-LOG")) ) {
			strcpy(LOG,argv[i+1]);
			strcpy(fname3,argv[i+2]);
			i+=3;
		}
		else
			i++;
	}

	//reconstructed file load
	if(!strcmp(form1,"bmp")){
		strcat(fname1,".bmp");
		Image1 = (struct Image_s *)LoadBmp(fname1);
		numCmpts1=Image1->row1step;
	}
	else if(!strcmp(form1,"tif")){
		strcat(fname1,".tif");
		Image1 = (struct Image_s *)LoadTif(fname1);
		if(Image1==NULL){
			printf("reconstruced file (tif-file) open error!\n");
			exit(0);
		}
		numCmpts1=Image1->col1step/Image1->width;
	}
	else if(!strcmp(form1,"ppm")){
		strcat(fname1,".ppm");
		Image1 = (struct Image_s *)LoadPpm(fname1);
		if(Image1==NULL){
			printf("reconstruced file (ppm-file) open error!\n");
			exit(0);
		}
		numCmpts1=Image1->row1step;
	}
	else if(!strcmp(form1,"raw")){
		strcat(fname1,".raw");
		if(!HDphoto1)
			Image1 = (struct Image_s *)LoadRAW(fname1, numCmpts1, numBitDipth1, width1, height1);
		else
			Image1 = (struct Image_s *)LoadRAW_HDphoto(fname1, numCmpts1, numBitDipth1, width1, height1);

		if(Image1==NULL){
			printf("reconstruced file (raw-file) open error!\n");
			exit(0);
		}
	}
	addr = new byte4 [2];
	addr[0]=0;
	addr[1]=0;
	diff = new byte4 [2*3];
	memset(diff, 0, sizeof(byte4)*6);

	//Original file load
	if(!strcmp(form2,"bmp")){
		strcat(fname2,".bmp");
		Image2 = (struct Image_s *)LoadBmp(fname2);
		numCmpts2=Image2->row1step;
	}
	else if(!strcmp(form2,"tif")){
		strcat(fname2,".tif");
		Image2 = (struct Image_s *)LoadTif(fname2);
		if(Image2==NULL){
			printf("original file (tif-file) open error!\n");
			exit(0);
		}
		numCmpts2=Image2->col1step/Image2->width;
	}
	else if(!strcmp(form2,"ppm")){
		strcat(fname2,".ppm");
		Image2 = (struct Image_s *)LoadPpm(fname2);
		if(Image2==NULL){
			printf("original file(ppm-file) open error!\n");
			exit(0);
		}
		numCmpts2=Image2->row1step;
	}
	else if(!strcmp(form2,"raw")){
		strcat(fname2,".raw");
		if(!HDphoto2)
			Image2 = (struct Image_s *)LoadRAW(fname2, numCmpts2, numBitDipth2, width2, height2);
		else
			Image2 = (struct Image_s *)LoadRAW_HDphoto(fname2, numCmpts2, numBitDipth2, width2, height2);

		if(Image2==NULL){
			printf("raw-file open error!\n");
			exit(0);
		}
	}

	//compare
	if(numCmpts1!=numCmpts2){
		printf("number of components is not equal! \n");
		exit(0);
	}

	d=getdistortion(Image1, Image2, numCmpts1, METRIC1, addr, diff);
	printf("Distortion=%f, x=%d, y=%d, Ref=,(,%x,%x,%x,),Target=,(,%x,%x,%x,)\n",d, addr[0], addr[1], diff[0], diff[1],diff[2], diff[3], diff[4], diff[5]);


	if( (!strcmp(LOG,"ON")) ||  (!strcmp(LOG,"on")) ){
		if((fp=fopen(fname3,"rb")) == NULL){
			printf("code stream file open error!\n");
			len=0;
			exit(0);
		}
		else{
			len=0;
			while(getc(fp)!=EOF){
				len++;
			}
			fclose(fp);
		}
		fp=fopen("log.csv","a+");
		fprintf(fp,"%s,%lf,%d\n",fname3, d, len);
		fclose(fp);
	}
	return	EXIT_SUCCESS;
}

/******************************************************************************\
* Distortion metric computation functions.
\******************************************************************************/

float getdistortion(struct Image_s *Image1, /*char metric1,*/ struct Image_s *Image2, /*char metric2,*/ byte2 numCmpts, char *metric, byte4 *addr, byte4 *diff)
{
	float	d;
	char	depth;
	
	if((!strcmp(metric,"PSNR")) || (!strcmp(metric,"psnr")) ){
		if(Image1->type==CHAR)			depth=8;
		else if(Image1->type==BYTE2)	depth=16;
		else if(Image1->type==BYTE4)	depth=32;
		d = psnr(Image1, /*metric1,*/ Image2, /*metric2,*/ numCmpts, depth);
	}
	else if((!strcmp(metric,"mse")) || (!strcmp(metric,"MSE")) ){
		byte8	Dis;
		Dis = mse(Image1, Image2, numCmpts);
		d = (float)Dis /(float)(Image1->width*Image1->height*numCmpts);
		d = (float)sqrt(d);
	}
	else if((!strcmp(metric,"norm")) || (!strcmp(metric,"NORM")) ){
		d = norm(Image1, /*metric1,*/ Image2, /*metric2,*/ numCmpts);

	}
	else if((!strcmp(metric,"pae")) || (!strcmp(metric,"PAE")) ){
		byte4	Dis;
		Dis = pae(Image1, /*metric1,*/ Image2, /*metric2,*/ numCmpts, addr, diff);
		d = (float)Dis;
	}
	else{
		printf("metric error!\n");
		return	EXIT_FAILURE;
	}
	return d;
}

/* Compute peak absolute error. */
byte4 pae(struct Image_s *Image1, /*char metric1,*/ struct Image_s *Image2, /*char metric2,*/ byte2 numCmpts, byte4 *addr, byte4 *diff)
{
	byte4	s, d;
	byte4	i, j;
	byte2	ccc;

	if(Image1->type==CHAR){
		uchar *D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (uchar *)Image1->data;
		D2_TS = (uchar *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
				for(ccc=0;ccc<numCmpts;ccc++){
					d = abs(D1[ccc] - D2[ccc]);
					if (d > s){
						s = d;
						addr[0] = i;
						addr[1] = j;
						diff[0] = (byte4)D1[0];
						diff[1] = (byte4)D1[1];
						diff[2] = (byte4)D1[2];
						diff[3] = (byte4)D2[0];
						diff[4] = (byte4)D2[1];
						diff[5] = (byte4)D2[2];
					}
				}
			}
		}
	}
	else if(Image1->type==BYTE2){
		byte2	*D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (byte2 *)Image1->data;
		D2_TS = (byte2 *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
				for(ccc=0;ccc<numCmpts;ccc++){
					d = abs(D1[ccc] - D2[ccc]);
					if(d>=256)
						i=i;
					if (d > s){
						s = d;
						addr[0] = i;
						addr[1] = j;
						diff[0] = (byte4)D1[0];
						diff[1] = (byte4)D1[1];
						diff[2] = (byte4)D1[2];
						diff[3] = (byte4)D2[0];
						diff[4] = (byte4)D2[1];
						diff[5] = (byte4)D2[2];
					}
				}
			}
		}
	}
	else if(Image1->type==BYTE4){
		byte4 *D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (byte4 *)Image1->data;
		D2_TS = (byte4 *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
//				if(metric1==metric2){
					for(ccc=0;ccc<numCmpts;ccc++){
						d = abs(D1[ccc] - D2[ccc]);
						if (d > s){
							s = d;
							addr[0] = i;
							addr[1] = j;
							diff[0] = (byte4)D1[0];
							diff[1] = (byte4)D1[1];
							diff[2] = (byte4)D1[2];
							diff[3] = (byte4)D2[0];
							diff[4] = (byte4)D2[1];
							diff[5] = (byte4)D2[2];
						}
					}
/*				}
				else{
					for(ccc=0;ccc<numCmpts;ccc++){
						d = abs(D1[ccc] - D2[numCmpts-1-ccc]);
						if (d > s){
							s = d;
							addr[0] = i;
							addr[1] = j;
							diff[0] = (byte4)D1[ccc];
							diff[1] = (byte4)D2[numCmpts-1-ccc];
						}
					}
				}*/
			}
		}
	}

	return s;
}

/* Compute either mean-squared error or mean-absolute error. */
byte8 mse(struct Image_s *Image1, /*char metric1,*/ struct Image_s *Image2, /*char metric2,*/ byte2 numCmpts)
{
	byte8	s;
	byte4	d;
	byte4	i, j;
	byte2	ccc;


	if(Image1->type==BIT1){
		byte4	xbyte;
		uchar *D1,*D2, *D1_TS, *D2_TS;
		uchar	tempD2, tempD1;
		D1_TS = (uchar *)Image1->data;
		D2_TS = (uchar *)Image2->data;
		xbyte=Image1->tbx1-Image1->tbx0;
		xbyte = ceil2(xbyte, 8);
		s=0;
		if(Image1->MaxValue==Image2->MaxValue){
			for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
				D1 = D1_TS;
				D2 = D2_TS;
				for (i=0; i < xbyte; i++, D1+=Image1->row1step, D2+=Image2->row1step){
					d = (byte4)abs(*D1 - *D2);
					s = s + (d*d);
				}
			}
		}
		else{
			for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
				D1 = D1_TS;
				D2 = D2_TS;
				for (i=0; i <xbyte ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
					tempD1 = *D1;
					tempD2 = ~(*D2);
					d = (byte4)abs(tempD1 - tempD2);
					s = s + (d*d);
				}
			}
		}
	}
	else if(Image1->type==CHAR){
		uchar *D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (uchar *)Image1->data;
		D2_TS = (uchar *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
				for(ccc=0;ccc<numCmpts;ccc++){
					d = (byte4)abs(D1[ccc] - D2[ccc]);
					s = s + (d*d);
				}
			}
		}
	}
	else if(Image1->type==BYTE2){
		ubyte2 *D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (ubyte2 *)Image1->data;
		D2_TS = (ubyte2 *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
				for(ccc=0;ccc<numCmpts;ccc++){
					d = (byte4)abs(D1[ccc] - D2[ccc]);
					s = s + (d*d);
				}
			}
		}
	}
	else if(Image1->type==BYTE4){
		byte4 *D1,*D2, *D1_TS, *D2_TS;
		D1_TS = (byte4 *)Image1->data;
		D2_TS = (byte4 *)Image2->data;
		s=0;
		for (j=Image1->tby0 ; j<Image1->tby1 ; j++, D1_TS+=Image1->col1step, D2_TS+=Image2->col1step){
			D1 = D1_TS;
			D2 = D2_TS;
			for (i=Image1->tbx0; i < Image1->tbx1 ; i++, D1+=Image1->row1step, D2+=Image2->row1step){
				for(ccc=0;ccc<numCmpts;ccc++){
					d = (byte4)abs(D1[ccc] - D2[ccc]);
					s = s + (d*d);
				}
			}
		}
	}

	return s;
}

float norm(struct Image_s *Image1, /*char metric1,*/ struct Image_s *Image2, /*char metric2,*/ byte2 numCmpts)
{
	byte8	Dis;
	long double m0;
	float	m1;

	Dis = mse(Image1, /*metric1,*/ Image2, /*metric2,*/ numCmpts);
	m0 = (long double)Dis;
	m1 = (float)sqrt(m0);
	return m1;
}


/* Compute peak signal-to-noise ratio. */

float psnr(struct Image_s *Image1, /*char metric1,*/ struct Image_s *Image2, /*char metric2,*/ byte2 numCmpts, byte4 depth)
{
	byte8	s, m0;
	byte4	p;
	float	m;
	s = mse(Image1, /*metric1,*/ Image2, /*metric2,*/ numCmpts);
	m0 = s / (Image1->width*Image1->height);

	if(m0==0)	return	0;
	else{
		m = (float)m0;
		m = (float)sqrt(m);
		p = ((1 << depth) - 1);
		return (float)(20.0 * (float)log10((float)p / m));
	}
}

/******************************************************************************\
*
\******************************************************************************/

void	*LoadRAW_HDphoto(char *fname, byte2 numCmpts, byte2 numBitDipth, byte4 width1, byte4 height1)
{
	FILE	*fp;
	byte4	xwidth0, xwidth;
	byte4	i,j;
	struct	Image_s *Image;
	uchar	*ImageD1;



	fp = fopen( fname, "rb");
	
	xwidth0 = numCmpts*width1;

	if(xwidth0%4)	xwidth = (xwidth0/4+1)*4;
	else			xwidth = xwidth0;
	Image = new	struct Image_s;
	Image->tbx0 = 0;
	Image->tbx1 = width1;
	Image->tby0 = 0;
	Image->tby1 = height1;
	Image->col1step = xwidth;
	Image->row1step = numCmpts;
	ImageD1 = new uchar [xwidth * height1];
	if(numCmpts==3){
		for(j=0;j<height1;j++){
			for(i=0;i<width1;i++){
				ImageD1[j*xwidth + 3*i+ 2] = getc(fp);
				ImageD1[j*xwidth + 3*i+ 1] = getc(fp);
				ImageD1[j*xwidth + 3*i+ 0] = getc(fp);
			}
		}
	}
	else{
		for(j=0;j<Image->tby1;j++){
			for(i=0;i<Image->tbx1;i++){
				ImageD1[j*xwidth + i] = getc(fp);
			}
		}
	}
	fclose(fp);
	Image->data = (byte4 *)ImageD1;
	Image->type = CHAR;

	return	(void *)Image;
}

void usage(void)
{
	fprintf(stderr, "usage:\n");
	fprintf(stderr, "[-t] [original file without extension] [-f] [original format(extension)] \n");
	fprintf(stderr, "[-T] [reconstructed file name without extension] [-F] [reconstructed format(extension)] [-log]\n");
	fprintf(stderr,
	  "[-m] The metric argument may assume one of the following values:\n"
	  "    psnr .... peak signal to noise ratio\n"
	  "    mse ..... mean squared error\n"
	  "    norm .... squared error\n"
	  "    rmse .... root mean squared error\n"
	  "    pae ..... peak absolute error\n"
	  "    mae ..... mean absolute error\n"
	  "    equal ... equality (boolean)\n"
	  );
	fprintf(stderr,"[-log] on :Create log.csv file by append mode\n");
	fprintf(stderr,"[-log] off:Do not create log.csv\n");
	exit(1);
}
