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
//#include	<math.h>
#include	<string.h>
//#include	<malloc.h>
#include	"ImageUtil.h"
#include	"Jb2_T4T6Lapper.h"


struct Jb2HuffmanTable_s *CreateHuffmanTable( char encdec )
{
	struct	Jb2HuffmanTable_s *Huff;
	byte4	iii, kkk;
	byte4	CodeLength;
	byte4	numD;
	uchar	No_Huff;
	Huff = new struct Jb2HuffmanTable_s [16];

	if(encdec==DEC){
		//Table_A
		No_Huff=0;
		Huff[No_Huff].numCode = numD = 4;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_A[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_A[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_A[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_A[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_A[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_A[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_B
		No_Huff=1;
		Huff[No_Huff].numCode = numD = 7;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_B[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_B[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_B[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_B[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_B[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_B[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}
		//Table_C
		No_Huff=2;
		Huff[No_Huff].numCode = numD = 9;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_C[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_C[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_C[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_C[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_C[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_C[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_D
		No_Huff=3;
		Huff[No_Huff].numCode = numD = 6;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_D[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_D[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_D[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_D[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_D[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_D[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_E
		No_Huff=4;
		Huff[No_Huff].numCode = numD = 8;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_E[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_E[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_E[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_E[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_E[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_E[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_F
		No_Huff=5;
		Huff[No_Huff].numCode = numD = 14;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_F[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_F[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_F[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_F[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_F[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_F[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_G
		No_Huff=6;
		Huff[No_Huff].numCode = numD = 15;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_G[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_G[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_G[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_G[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_G[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_G[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_H
		No_Huff=7;
		Huff[No_Huff].numCode = numD = 21;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_H[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_H[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_H[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_H[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_H[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_H[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_I
		No_Huff=8;
		Huff[No_Huff].numCode = numD = 22;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_I[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_I[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_I[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_I[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_I[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_I[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_J
		No_Huff=9;
		Huff[No_Huff].numCode = numD = 21;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_J[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_J[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_J[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_J[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_J[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_J[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_K
		No_Huff=10;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_K[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_K[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_K[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_K[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_K[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_K[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_L
		No_Huff=11;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_L[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_L[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_L[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_L[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_L[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_L[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_M
		No_Huff=12;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_M[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_M[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_M[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_M[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_M[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_M[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}	

		//Table_N
		No_Huff=13;
		Huff[No_Huff].numCode = numD = 5;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_N[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_N[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_N[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_N[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_N[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_N[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}

		//Table_O
		No_Huff=14;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].DecC       = new byte4 [numD];
		Huff[No_Huff].DecC_L     = new byte4 [numD];
		Huff[No_Huff].DecBits    = new byte4 [numD];
		Huff[No_Huff].DecVal     = new byte4 [numD];
		Huff[No_Huff].DecValFunc = new byte4 [numD];
		memset( Huff[No_Huff].DecC, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecC_L, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecBits, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecVal, 0, sizeof(byte4)*numD);
		memset( Huff[No_Huff].DecValFunc, 0, sizeof(byte4)*numD);
		CodeLength=1;
		iii=0;
		while(iii<numD){
			for(kkk=0 ; kkk<numD ; kkk++ ){
				if(CodeLength==(byte4)Huffman_Table_O[1][kkk]){
					Huff[No_Huff].DecC[iii]      =Huffman_Table_O[0][kkk];
					Huff[No_Huff].DecC_L[iii]    =Huffman_Table_O[1][kkk];
					Huff[No_Huff].DecBits[iii]   =Huffman_Table_O[2][kkk];
					Huff[No_Huff].DecVal[iii]    =Huffman_Table_O[3][kkk];
					Huff[No_Huff].DecValFunc[iii]=Huffman_Table_O[4][kkk];
					iii++;
				}
			}
			CodeLength++;			
		}
	}
	else{
		//Table_A
		No_Huff=0;
		Huff[No_Huff].numCode = numD = 4;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_A[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_A[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_A[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_A[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_A[4], sizeof(byte4)*numD);

		//Table_B
		No_Huff=1;
		Huff[No_Huff].numCode = numD = 7;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_B[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_B[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_B[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_B[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_B[4], sizeof(byte4)*numD);

		//Table_C
		No_Huff=2;
		Huff[No_Huff].numCode = numD = 9;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_C[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_C[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_C[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_C[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_C[4], sizeof(byte4)*numD);

		//Table_D
		No_Huff=3;
		Huff[No_Huff].numCode = numD = 6;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_D[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_D[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_D[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_D[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_D[4], sizeof(byte4)*numD);

		//Table_E
		No_Huff=4;
		Huff[No_Huff].numCode = numD = 8;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_E[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_E[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_E[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_E[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_E[4], sizeof(byte4)*numD);

		//Table_F
		No_Huff=5;
		Huff[No_Huff].numCode = numD = 14;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_F[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_F[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_F[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_F[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_F[4], sizeof(byte4)*numD);

		//Table_G
		No_Huff=6;
		Huff[No_Huff].numCode = numD = 15;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_G[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_G[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_G[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_G[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_G[4], sizeof(byte4)*numD);

		//Table_H
		No_Huff=7;
		Huff[No_Huff].numCode = numD = 21;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_H[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_H[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_H[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_H[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_H[4], sizeof(byte4)*numD);

		//Table_I
		No_Huff=8;
		Huff[No_Huff].numCode = numD = 22;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_I[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_I[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_I[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_I[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_I[4], sizeof(byte4)*numD);

		//Table_J
		No_Huff=9;
		Huff[No_Huff].numCode = numD = 21;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_J[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_J[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_J[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_J[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_J[4], sizeof(byte4)*numD);

		//Table_K
		No_Huff=10;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_K[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_K[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_K[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_K[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_K[4], sizeof(byte4)*numD);

		//Table_L
		No_Huff=11;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_L[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_L[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_L[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_L[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_L[4], sizeof(byte4)*numD);

		//Table_M
		No_Huff=12;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_M[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_M[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_M[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_M[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_M[4], sizeof(byte4)*numD);

		//Table_N
		No_Huff=13;
		Huff[No_Huff].numCode = numD = 5;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_N[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_N[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_N[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_N[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_N[4], sizeof(byte4)*numD);

		//Table_O
		No_Huff=14;
		Huff[No_Huff].numCode = numD = 13;
		Huff[No_Huff].EncC       = new byte4 [numD];
		Huff[No_Huff].EncC_L     = new byte4 [numD];
		Huff[No_Huff].EncBits    = new byte4 [numD];
		Huff[No_Huff].EncVal     = new byte4 [numD];
		Huff[No_Huff].EncValFunc = new byte4 [numD];
		memcpy( Huff[No_Huff].EncC, &Huffman_Table_O[0], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncC_L, &Huffman_Table_O[1], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncBits, &Huffman_Table_O[2], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncVal, &Huffman_Table_O[3], sizeof(byte4)*numD);
		memcpy( Huff[No_Huff].EncValFunc, &Huffman_Table_O[4], sizeof(byte4)*numD);
	}
	return	Huff;
}

byte4 JBIG2_ID_Dec( byte4 *C, byte4 *L, byte4 *O, struct StreamChain_s *str )
{
	byte4	kkk, data, length_K, length_Last;
	data=0;
	length_Last=0;
	kkk=-1;
	do{
		kkk++;
		length_K = L[O[kkk]]-length_Last;
		if(length_K)
			data = (byte4 )( (data<<length_K) | Ref_nBits(str, length_K) );
		length_Last = L[O[kkk]];
	} while( data!=C[O[kkk]] );
	return	O[kkk];
}

struct StreamChain_s *JBIG2_ID_Enc( byte4 Val, struct Jb2HuffmanTable_s *Huff, struct StreamChain_s *str )
{

	return	str;
}

byte4	JBIG2_HuffDec(struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff)
{
	byte4	Val;
	byte4	kkk;
	byte4	data;
	byte4	length_K, length_Last;
	
	kkk=-1;
	length_Last=0;
	data=0;
	do{
		kkk++;
		length_K = Huff->DecC_L[kkk]-length_Last;
		if(length_K)
			data = (byte4 )( (data<<length_K) | Ref_nBits(str, length_K) );
		length_Last = Huff->DecC_L[kkk];
		if(kkk>256){
			printf("ERROR!\n");
			exit(0);
		}
	} while( data != Huff->DecC[kkk] );

	switch(Huff->DecValFunc[kkk]){
	case 0:
		Val = Huff->DecVal[kkk] + (byte4)Ref_nBits(str, Huff->DecBits[kkk]);
		break;
	case 2:
		Val = Huff->DecVal[kkk] - (byte4)Ref_nBits(str, Huff->DecBits[kkk]) ;
		break;
	default:
		break;
	}
	return	Val;
}

struct StreamChain_s *JBIG2_HuffEnc( byte4 Val, struct StreamChain_s *str, struct Jb2HuffmanTable_s *Huff)
{
	byte4	Prefix_Val;
	byte4	kkk;
	char	Flag=0;

	if(Huff->EncVal[Huff->numCode-1]==OOB)
		Flag=1;

	if(Val==OOB){
		StreamBitWrite( Huff->EncC[Huff->numCode-1], Huff->EncC_L[Huff->numCode-1], str, str->buf_length);
		return	str;
	}

	for( kkk=1 ; kkk<(Huff->numCode-Flag) ; kkk++ ){
		if(Val<Huff->EncVal[kkk])
			break;
	}
	kkk--;

	switch(Huff->EncValFunc[kkk]){
	case 0:
		Prefix_Val = Val - Huff->EncVal[kkk];
		break;
	case 2:
		Prefix_Val = -Val + Huff->EncVal[kkk];
		break;
	default:
		break;
	}
#if 0
	printf("%d,%d,%d,%d,%d,%d,%d,%d\n",Val, kkk, Huff->EncVal[kkk], Huff->EncValFunc[kkk], Prefix_Val, Huff->EncC[kkk], Huff->EncC_L[kkk], Huff->EncBits[kkk]);
#endif
	str = StreamBitWrite(Huff->EncC[kkk], Huff->EncC_L[kkk], str, str->buf_length);
	str = StreamBitWrite(Prefix_Val, Huff->EncBits[kkk], str, str->buf_length);
	return	str;
}
