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





#ifndef MQ_LAPPER_H
#define MQ_LAPPER_H

#include	"MQ_codec.h"

void	InitMQ_Codec2( struct mqcodec_s *codec );

struct	StreamChain_s *MQ_EncInteger( byte4 V, struct StreamChain_s *str, struct mqcodec_s *codec, byte4 bCX );
byte4	MQ_DecInteger( struct StreamChain_s *str, struct mqcodec_s *codec, byte4 bCX, byte4 Code_Length, uchar *Flag );

struct	StreamChain_s *MQ_EncIntegerIAID( byte4 Val, struct StreamChain_s *str, struct mqcodec_s *codec, byte4 SymbolCodeLength, byte4 bCX );
byte4	MQ_DecIntegerIAID( struct StreamChain_s *str, struct mqcodec_s *codec, byte4 Code_Length, byte4 SymbolCodeLength, byte4 bCX );

struct	StreamChain_s *MQ_EncImage( struct Image_s *Image, struct StreamChain_s *str, struct mqcodec_s *codec, char TpGDon, char Template, char ExtTemplate, char ATX1, char ATY1, char ATX2, char ATY2, char ATX3, char ATY3, char ATX4, char ATY4, char ATX5, char ATY5, char ATX6, char ATY6, char ATX7, char ATY7, char ATX8, char ATY8, char ATX9, char ATY9, char ATX10, char ATY10, char ATX11, char ATY11, char ATX12, char ATY12, char DebugF );
struct Image_s *MQ_DecImage( byte4 width, byte4 height, struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Code_Length, char TpGDon, char Template, char ExtTemplate, char ATX1, char ATY1, char ATX2, char ATY2, char ATX3, char ATY3, char ATX4, char ATY4, char ATX5, char ATY5, char ATX6, char ATY6, char ATX7, char ATY7, char ATX8, char ATY8, char ATX9, char ATY9, char ATX10, char ATY10, char ATX11, char ATY11, char ATX12, char ATY12);

struct StreamChain_s *MQ_RefinementEncImage( struct Image_s *RefImage, struct Image_s *ImageC, byte4 RefDx, byte4 RefDy, struct mqcodec_s *codec, struct StreamChain_s *str, char TpGDon, char rTemplate, char rATX1, char rATY1, char rATX2, char rATY2);
struct	Image_s *MQ_RefinementDecImage( struct Image_s *RefImage, byte4 width, byte4 height, byte4 RefDx, byte4 RefDy, struct mqcodec_s *codec, struct StreamChain_s *str, byte4 Code_Length, char TpGDon, char rTemplate, char ATX1, char ATY1, char rATX2, char rATY2);

byte4	CX_Encode( byte4 j, byte4 i, byte4 width,  struct Image_s *Image, uchar *D2_, uchar *D1_, uchar *D0_, uchar ExtTemplate, char ATX1, char ATX2, char ATX3, char ATX4, char ATX5, char ATX6, char ATX7, char ATX8, char ATX9, char ATX10, char ATX11, char ATX12, uchar *A1_, uchar *A2_, uchar *A3_, uchar *A4_, uchar *A5_, uchar *A6_, uchar *A7_, uchar *A8_, uchar *A9_, uchar *A10_, uchar *A11_, uchar *A12_);
byte4	CX_RefEncode( byte4 i, byte4 ref_i, byte4 width, byte4 ref_width, uchar *rD2_, uchar *rD1_, uchar *rD0_, uchar *D1_, uchar *D0_, uchar rTemplate, uchar *A1_, uchar *rA2_, char ATX1, char ATX2 );

#endif

