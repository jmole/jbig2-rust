# ITU-T Rec. T.88 (08/2018) -- JBIG2 Reference Package

This directory is a verbatim copy of the electronic attachments published with
**Recommendation ITU-T T.88 | ISO/IEC 14492, Edition 2 (2018-08-29)**, commonly
known as **JBIG2**. It bundles:

1. The normative specification text (`ITU-T_T_88__08_2018.docx` /
   `ITU-T_T_88__08_2018.pdf`).
2. The **reference C/C++ sample software** for a JBIG2 encoder, decoder,
   auxiliary arithmetic-coder tester, and image-comparison tool, under
   `Software/JBIG2_SampleSoftware-A20180829/`.
3. The **conformance test data** (reference bitmaps + reference codestreams)
   under `Software/JBIG2_ConformanceData-A20180829/`.

The material is provided under the ICT-Link copyright licence in
`Software/Copyright Notice.txt` -- it grants an irrevocable, worldwide,
royalty-free licence to reproduce, modify, and distribute the software for the
limited purpose of implementing and testing JBIG2 / ISO/IEC 14492 conforming
decoders and encoders. No patent licence is granted.

The rest of this document is a practical map of the package: what JBIG2 is,
the directory layout, the build system, the command-line tools, the
conformance test vectors, and a module-by-module tour of the source tree.

---

## 1. What JBIG2 is (from the specification)

JBIG2 is a coding method for **bi-level images** (black-and-white printed
matter, faxes, scanned documents). Pixels are 0 or 1 and any greyscale/colour
interpretation is left to the application. Highlights from the spec:

- Supports **lossless, lossy, and progressive** coding of single images and
  multi-page documents.
- Particularly good on mixtures of **text and dithered (halftone) data**.
- A page is broken into **regions**, each coded with one of four region
  procedures:
  - **Generic region**: pixel-by-pixel arithmetic (MQ) coding with a small
    neighbourhood template, or MMR Huffman coding of runs.
  - **Generic refinement region**: codes a bitmap relative to a reference
    bitmap using an arithmetic refinement template.
  - **Symbol/Text region**: draws previously defined symbols from a
    **symbol dictionary**, optionally refining each instance.
  - **Halftone region**: paints a fixed set of **patterns** from a
    **pattern dictionary** onto a regular grid.
- Two entropy coders: the IBM/ISO **MQ arithmetic coder** (Annex E) and a set
  of **Huffman tables** (Annex B).
- Streams are divided into **segments** with a common segment header
  (Clause 7.2). The segment types are enumerated in `Jb2Common.h`:
  `SYMBOL_DICTIONARY` (0x00), `IMMEDIATE_TEXT_REGION` (0x06), ...,
  `IMMEDIATE_GENERIC_REGION` (0x26), `PAGE_INFORMATION` (0x30),
  `END_OF_PAGE` (0x31), `END_OF_FILE` (0x33), `TABLES` (0x35),
  `COLOUR_PALETTE` (0x36), `EXTENSION` (0x3e), etc.
- A JBIG2 file starts with the 8-byte magic `97 4A 42 32 0D 0A 1A 0A`
  followed by a flags byte and a 32-bit page count (see
  `JB2_FILE_HEADER_ID` in `Jb2Common.h`).
- The 2018 edition incorporates AMD1/2/3 (Amendments 1-3, e.g. extended
  generic-region templates with up to 12 adaptive pixels and coloured
  text regions with a palette).

Edition history (from the document):

| Edition | Document              | Approval    |
| ------- | --------------------- | ----------- |
| 1.0     | ITU-T T.88            | 2000-02-10  |
| 1.1     | T.88 Amd.1            | 2003-06-29  |
| 1.2     | T.88 Amd.2            | 2003-06-29  |
| 1.3     | T.88 Amd.3            | 2011-05-14  |
| **2.0** | **ITU-T T.88 (2018)** | 2018-08-29  |

---

## 2. Directory layout

```
vendor/T-REC-T.88-201808/
|-- ITU-T_T_88__08_2018.docx        # Normative text (Word form)
|-- ITU-T_T_88__08_2018.pdf         # Normative text (PDF form, not read here)
|-- SUMMARY.md                      # This file
`-- Software/
    |-- Copyright Notice.txt        # ICT-Link licence
    |-- JBIG2_ConformanceData-A20180829/   # Frozen, official test vectors
    |   |-- F01_200.bmp             # Large source bitmap (IEEE F01 fax page)
    |   |-- F01_200_TT9.jb2         # Reference JBIG2 for test 9 (Huffman)
    |   |-- F01_200_TT9_TT00.bmp    # Reference decoded bitmap, page 0
    |   |-- F01_200_TT10.jb2        # Reference JBIG2 for test 10 (arithmetic)
    |   |-- F01_200_TT10_TT00.bmp
    |   |-- codeStreamTest1.bmp     # Tiny 32x32-class canvas used by tests 1-5, 7
    |   |-- codeStreamTest1_TT1.jb2 # Multi-page test vector
    |   |-- codeStreamTest1_TT1_TT00.bmp / _TT01.bmp / _TT02.bmp   # 3 decoded pages
    |   |-- codeStreamTest1_TT2.jb2 .. _TT5.jb2, _TT7.jb2          # + decoded BMPs
    |   |-- codeStreamTest2.bmp     # Very small canvas for test 6 (TR refinement)
    |   |-- codeStreamTest2_TT6.jb2
    |   |-- codeStreamTest2_TT6_TT00.bmp
    |   |-- codeStreamTest3.bmp     # Colour-palette test canvas
    |   |-- codeStreamTest3_TT8.jb2 # Text Region with palette (AMD3)
    |   `-- codeStreamTest3_TT8_TT00.bmp
    `-- JBIG2_SampleSoftware-A20180829/
        |-- Makefile                # Top-level (build + test)
        |-- source/                 # All C/C++ sources and headers (see sec. 5)
        |   |-- Makefile            # Compiles jbig2, jb2z1 (Z1), imgcomp
        |   |-- JBIG2_Main.cpp      # CLI entry for `jbig2` (enc/dec driver)
        |   |-- Jbig2ENC.cpp        # Top-level JBIG2 encoder
        |   |-- Jbig2DEC.cpp        # Top-level JBIG2 decoder
        |   |-- JBIG2Common.cpp     # Segment dispatch, parameter parsing, chains
        |   |-- Jb2Common.h         # Core JBIG2 segment/parameter structs
        |   |-- Jb2_MQLapper.cpp/.h # MQ integer decoders (IA*), image MQ coder
        |   |-- MQ_codec.cpp/.h     # Bit-level MQ arithmetic coder (Annex E)
        |   |-- Jb2_T4T6Lapper.cpp/.h  # JBIG2 Huffman tables A..O (Annex B)
        |   |-- T4T6codec.h         # Classic T.4/T.6 MH/MR/MMR codec helpers
        |   |-- codsub.cpp          # T.4/T.6 encoder (MH/MR/MMR line coder)
        |   |-- decsub.cpp          # T.4/T.6 decoder
        |   |-- T45_codec.cpp/.h    # T.44/T.45 colour-palette codec glue
        |   |-- ImageUtil.cpp/.h    # BMP/TIFF/PPM/RAW I/O + bitmap helpers
        |   |-- imgcomp.cpp/.h      # `imgcomp` distortion tool (MSE/PSNR/PAE/NORM)
        |   |-- Jbig2_Z1main.cpp    # `jb2z1` (Z1) self-test for MQ / Huffman
        |   |-- Jb2_Debug.cpp/.h    # Optional image-dump debug hooks
        |   `-- asmasm.h            # Tiny platform-feature toggle header
        |-- test/
        |   |-- Makefile            # The 10 conformance tests (T1..T10)
        |   |-- JBIG22.bat          # Windows equivalent of the test Makefile
        |   |-- jbig2_Param2.ini .. jbig2_Param9.ini, jbig2_Param100.ini
        |   |                       # Encoder parameter files for each TTn test
        |   |-- codeStreamTest1.bmp, codeStreamTest2.bmp, codeStreamTest3.bmp,
        |   |-- F01_200.bmp         # Inputs (same as conformance data)
        |   |-- codeStreamTest1_TT1.jb2  # Reference pre-made codestream (test 1)
        |   `-- Sym*.bmp, AggImage*.bmp, ImageResi*.bmp
        |                           # Small symbol-dictionary bitmaps referenced
        |                           # by the .ini files and by Jb2_Z1main.cpp
        |-- JBIG2/, imgcomp/, Z1/   # MSVC (Visual Studio 6) .dsp/.dsw/.ncb
        |                             project files for each executable.
        |                             Not used by the gcc/make flow.
        `-- Debug/, Release/        # MSVC output dirs (empty in this copy)
```

On macOS/Linux only the top-level `Makefile` and the `source/` and `test/`
subtrees matter; the `.dsw`/`.dsp`/`.ncb`/`.opt` files are Microsoft Visual
Studio 6 project scaffolding for Windows builds.

---

## 3. Build system

### 3.1 Toolchain requirements

- C++ compiler (`$(CXX)`, e.g. `g++` or Apple `clang++`).
- `make`.
- Standard C math + POSIX threads (`-lm -lpthread`); the `Makefile` notes
  that `-lpthread` can be dropped if your `coresys` is not multithreaded.

The Makefile defines:

```startLine:endLine:vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/source/Makefile
INCLUDES = -I./ -I../include 

C_OPT = -DNDEBUG -Wall -Wno-uninitialized -Wno-deprecated $(KDU_GLIBS)

CFLAGS = $(INCLUDES) $(C_OPT) $(DEFINES)
LIBS = -lm -lpthread
```

C++ files are compiled with `-O2 -DNDEBUG`, C files with `-O2 -DNDEBUG
-std=c99`. Clang emits warnings about implicit `int`->`char` conversions
(constants such as `4321` being stored in a `char`) and some
`long double`/`double` format-specifier mismatches in `ImageUtil.cpp`; these
are benign and do not affect the built binaries.

### 3.2 Top-level `Makefile`

```1:21:vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/Makefile
#
# Makefile for JBIG2 reference software.
# by Junichi HARA

.PHONY: all jbig2 imgcomp test clean

all:	jbig2 imgcomp test

jbig2:
	@(cd source; make jbig2)

imgcomp:
	@(cd source; make imgcomp)

test:	jbig2 imgcomp
	@(cd test; make test)

clean:
	@(cd source; make clean)
	@(cd test; make clean)
```

### 3.3 `source/Makefile`

Three binaries are built **in the `source/` directory** (not in `Debug/`):

| Target     | Purpose                                              | Main source         |
| ---------- | ---------------------------------------------------- | ------------------- |
| `jbig2`    | JBIG2 encoder/decoder driver                         | `JBIG2_Main.cpp`    |
| `jb2z1`    | MQ / Huffman / refinement self-test (a.k.a. `Z1`)    | `Jbig2_Z1main.cpp`  |
| `imgcomp`  | Pixel-domain distortion comparator                   | `imgcomp.cpp`       |

The shared JBIG2 translation units (`JB2SRC` in the Makefile) are:

```
codsub.cpp  decsub.cpp  JBIG2Common.cpp  Jb2_Debug.cpp  Jb2_MQLapper.cpp
Jb2_T4T6Lapper.cpp  Jbig2DEC.cpp  Jbig2ENC.cpp  MQ_codec.cpp  T45_codec.cpp
```

and `IMGSRC = ImageUtil.cpp` is the image I/O and bit-packing helper library.
Each binary links its main against `JB2SRC` (except `imgcomp`, which only
needs `IMGSRC`).

### 3.4 `test/Makefile`

Runs the **10 conformance tests** (T1..T10) defined by Annex K.1.3 of the
spec. For each test it either decodes a reference `.jb2` or
encodes-then-decodes a reference `.bmp` and finally runs `imgcomp ... -m mse`;
all tests should report `Distortion=0.000000`.

### 3.5 Typical invocation

From the `JBIG2_SampleSoftware-A20180829/` directory:

```bash
make          # builds jbig2 + imgcomp, then runs all 10 tests
make jbig2    # build the encoder/decoder only
make imgcomp  # build the comparator only
make clean    # remove object files and binaries
```

`jb2z1` is built by `make` inside `source/` (it is part of the `all:` target
in `source/Makefile`) but is not exercised by the top-level `make test`.

> Verified locally: `make test` runs all 10 tests end-to-end with exit code
> 0 and `Distortion=0.000000` on every `imgcomp` invocation.

---

## 4. Command-line tools

### 4.1 `jbig2` -- encoder/decoder driver

Source: `source/JBIG2_Main.cpp`. The tool detects encode vs. decode from
the input file extension.

Usage (the program's own help is shown in
`JBIG2_Main.cpp` lines 78-96):

```
jbig2 -i <in_stem> -f <in_ext> -o <out_stem> -F <out_ext> [-ini <params.ini>]

-i <stem>    Input file name WITHOUT extension.
-o <stem>    Output file name WITHOUT extension.
-f <ext>     Input extension. Determines mode:
               bmp|BMP|Bmp  -> encode (BMP input)
               tif|TIF|tiff -> encode (TIFF input)
               img          -> encode (raw binary)
               jb2          -> decode (JBIG2 codestream input)
-F <ext>     Output extension.
               jb2          (when encoding)
               bmp | tif    (when decoding; one file per decoded page,
                             suffixed "00", "01", ...)
-ini <file>  Optional parameter file (encoder only). If omitted, the
             encoder emits a single generic-region page using default
             arithmetic-coding settings.
-W <n>       (reserved) explicit width for raw input.
-H <n>       (reserved) explicit height for raw input.
```

Examples (from `test/Makefile`):

```bash
# Decode a bundled codestream into per-page BMPs (multi-page output):
jbig2 -i codeStreamTest1_TT1 -f jb2 -o codeStreamTest1_TT1_TT -F bmp
# -> produces codeStreamTest1_TT1_TT00.bmp, _TT01.bmp, _TT02.bmp

# Encode with a parameter file that describes a symbol dictionary + text region:
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT2 -F jb2 -ini jbig2_Param2.ini
# Round-trip decode:
jbig2 -i codeStreamTest1_TT2 -f jb2 -o codeStreamTest1_TT2_TT -F bmp

# Default-encoder test (no .ini -> single generic-region page):
jbig2 -i F01_200 -f bmp -o F01_200_TT10 -F jb2
```

#### 4.1.1 Encoder `.ini` parameter files

Parameter files are plain ASCII, read by `Jb2ParamInit()` in
`source/JBIG2Common.cpp`. They describe each segment the encoder should emit,
in order, after the implicit PageInformation segment (segment 0). The general
grammar is: tokens beginning with `-` introduce sections, the rest are
values. See the `test/jbig2_Param*.ini` files and
`K.3.1.2 "Ini file format example"` in the spec for the canonical list.

Summary of section markers:

| Marker       | Meaning                                                    |
| ------------ | ---------------------------------------------------------- |
| `-sym`       | Symbol-dictionary segment definition                       |
| `-txt`       | Text-region segment definition                             |
| `-Gen`       | Generic-region segment definition                          |
| `-Seg N`     | Segment number (PageInfo is always 0, first user seg is 1) |
| `-file`      | Start input-dictionary data block (for `-sym`)             |
| `-Param`     | Start parameter block                                      |

Within `-sym -file ...` you describe height classes, widths, and the BMPs
of each symbol glyph (`-Simple n Sym00n.bmp`, `-Ref ...` for refinement
symbols). Within `-sym -Param ...` / `-txt -Param ...` / `-Gen -Param ...`
you set Huffman vs. arithmetic flags, adaptive-template pixels
(`-ATX1 ... -ATX12`, `-ATY1 ... -ATY12`), extended template
(`-ExtTemplate 1`), refinement templates, colour palettes
(`-ColorExt ...`), corner placement, symbol positions inside the text
region (`-ID id height_pos width_pos`), and region width/height
(`-W`, `-H`).

Each of the 10 tests in `test/Makefile` corresponds to a different .ini:

| Test | Input            | .ini file          | Feature exercised                                             |
| ---- | ---------------- | ------------------ | ------------------------------------------------------------- |
| T1   | pre-made `.jb2`  | --                 | Multi-page JBIG2 + Huffman coding (decode-only)               |
| T2   | codeStreamTest1  | jbig2_Param2.ini   | Huffman-coded symbol dictionary + text region                 |
| T3   | codeStreamTest1  | jbig2_Param3.ini   | Arithmetic-coded symbol dictionary + text region              |
| T4   | codeStreamTest1  | jbig2_Param4.ini   | Generic-region template = 1                                   |
| T5   | codeStreamTest1  | jbig2_Param5.ini   | Symbol Dictionary Segment **refinement** (AMD-era)            |
| T6   | codeStreamTest2  | jbig2_Param6.ini   | Text Region Segment symbol **refinement**                     |
| T7   | codeStreamTest1  | jbig2_Param7.ini   | **AMD2**: extended 12-AT generic-region template              |
| T8   | codeStreamTest3  | jbig2_Param8.ini   | **AMD3**: text region with colour palette (`-ColorExt ...`)   |
| T9   | F01_200          | jbig2_Param9.ini   | Whole page as a single Huffman-coded generic region           |
| T10  | F01_200          | --                 | Default encoder: arithmetic-coded generic region (no params)  |

`jbig2_Param100.ini` is a richer sample (a real symbol dictionary with
glyphs A-Z/a-z/digits using the `Sym100_*.bmp` files); it is not run by the
Makefile but is kept as a reference by the spec text.

### 4.2 `imgcomp` -- distortion measurement

Source: `source/imgcomp.cpp`. Compares two images pixel-by-pixel.

```
imgcomp -t <ref_stem> -f <ref_ext> -T <tgt_stem> -F <tgt_ext> -m <metric>
        [-log on|off <codestream_for_log>]

-t       Reference ("original") file stem, no extension
-T       Target    ("reconstructed") file stem, no extension
-f / -F  Extensions: bmp | tif | ppm | raw
         For raw: -f raw <W> <H> <numCmpts> <bitDepth> [-HDphoto]
-m       Metric: psnr | mse | norm | pae   (case-insensitive)
-log on  Append "<codestream>,<distortion>,<bytes>\n" to log.csv
-log off Do not write log.csv
```

The tool prints (see `imgcomp.cpp:231`):

```
Distortion=<value>, x=<xPAE>, y=<yPAE>, Ref=,(,r0,r1,r2,),Target=,(,t0,t1,t2,)
```

Only `Distortion` is significant for lossless bi-level conformance; the
`x,y,Ref,Target` fields are used by `pae` (peak absolute error) mode.
`usage()` in `imgcomp.cpp` enumerates supported metrics; `equal`, `rmse`,
`mae` are listed in the help text but only `psnr`, `mse`, `norm`, `pae`
are actually implemented -- the test Makefile only uses `mse`.

### 4.3 `jb2z1` -- internal self-test (Z1)

Source: `source/Jbig2_Z1main.cpp`. Takes no arguments. Exercises:

1. **Huffman tables A-O** (Annex B) by round-tripping random signed integers
   through `JBIG2_HuffEnc` / `JBIG2_HuffDec` and checking equality.
2. **MQ integer decoders** (`IADH`, `IADW`, `IADS`, `IADT`, `IAFS`, `IAIT`,
   `IADS`, `IARI`, ...) by round-tripping a hand-crafted sequence through
   `MQ_EncInteger` / `MQ_DecInteger`.
3. **MQ IAID** symbol-ID codec via `MQ_EncIntegerIAID`/`MQ_DecIntegerIAID`.
4. **MQ image encoder/decoder** on a hand-built 16x16 bitmap.
5. **MQ refinement encoder/decoder** using `Sym000.bmp` and `Sym001.bmp`
   as the reference / target bitmaps.

On success it prints "Table_(n) is OK", "MQ_Integer is OK", ...,
"MQ_EncImage/MQ_DecImage is OK", "MQ_RefinementEncImage/... is OK", and
finally "program end". It is a diagnostic aid, not part of the normal
conformance run.

---

## 5. Source tree in detail

All sources live in `Software/JBIG2_SampleSoftware-A20180829/source/`.
Approximate line counts (from `wc -l`):

| File                    | LoC   | Role                                                                 |
| ----------------------- | ----- | -------------------------------------------------------------------- |
| `ImageUtil.cpp`         | 3219  | BMP/TIFF/PPM/RAW readers & writers, bit<->byte packing, stream buffer|
| `ImageUtil.h`           | 674   | All shared typedefs (`byte4`, `uchar`, `Image_s`, `StreamChain_s`...)|
| `JBIG2Common.cpp`       | 1156  | `Jb2ParamInit` (INI parser), chain/segment helpers, TR helpers       |
| `Jb2Common.h`           | 603   | Segment type enum, segment/parameter structs (7.2-7.4 of spec)       |
| `Jbig2ENC.cpp`          | 968   | Encoder entry points, per-segment encode, ATs, refinement encoding   |
| `Jbig2DEC.cpp`          | 1664  | Decoder entry points, file-header parsing, per-segment decode        |
| `Jb2_MQLapper.cpp`      | 1006  | MQ integer decoders (IADH/IADW/IADS/IADT/IAFS/IAIT/IARI/IAID/...),   |
|                         |       | MQ image encoder/decoder, generic refinement image coder             |
| `Jb2_MQLapper.h`        |  79   | Public API of the MQ higher-level wrappers                           |
| `MQ_codec.cpp`          |  306  | Bit-level MQ encoder/decoder (Annex E -- `Enc_MQ`, `Dec_MQ`,         |
|                         |       | `MQ_ByteIn/Out`, `MQ_flush`, `InitMQ_Codec`, `QeIndexTable`)         |
| `MQ_codec.h`            |  145  | MQ codec struct & API + Qe probability state-machine table          |
| `Jb2_T4T6Lapper.cpp`    |  814  | JBIG2 Huffman tables A-O (Annex B.5) + `JBIG2_HuffEnc/Dec`           |
| `Jb2_T4T6Lapper.h`      |  190  | Static Huffman table definitions (value, code, length, bit-width)    |
| `codsub.cpp`            |  692  | T.4/T.6 (CCITT Group 3/4) **encoder** -- MH/MR/MMR run-length coder  |
| `decsub.cpp`            |  571  | T.4/T.6 **decoder**                                                  |
| `T4T6codec.h`           |  412  | MH/MR/MMR constants & run/make-up tables used by cod/decsub          |
| `T45_codec.cpp`         |  168  | T.44/T.45 colour-palette / run-length colour coder                   |
| `T45_codec.h`           |  74   | T.44/T.45 API                                                        |
| `JBIG2_Main.cpp`        |  248  | `jbig2` CLI front-end (arg parsing, file I/O dispatch)               |
| `imgcomp.cpp`           |  590  | `imgcomp` CLI front-end + metric implementations                     |
| `imgcomp.h`             |  66   | Metric prototypes                                                    |
| `Jbig2_Z1main.cpp`      |  407  | `jb2z1` self-test main                                               |
| `Jb2_Debug.cpp/.h`      | 103/74| Optional per-segment bitmap dumps (off by default except `DEBUG05`)  |
| `MQ_Lapper.h`           |  68   | Legacy placeholder header (entirely commented out)                   |
| `asmasm.h`              |  61   | Two feature toggles: `Cplus` (MSVC/`_MSC_VER` exports) and           |
|                         |       | `T1_THREAD`. Both default to 0.                                      |
| `Makefile`              |  83   | Per-source build rules                                               |

### 5.1 Layered architecture

```
                 +------------------------------------------+
 jbig2 CLI  ---> | JBIG2_Main.cpp  (arg parse, file I/O)    |
                 +------------------------------------------+
                              |
               +--------------+----------------------+
               v                                     v
     Jbig2ENC.cpp (JBIG2_EncMain)        Jbig2DEC.cpp (JBIG2_DecMain)
               |                                     |
               v                                     v
                  +------------------------------+
                  | JBIG2Common.cpp / Jb2Common.h|  segment structs,
                  | Jb2ParamInit, SegmentEncode, |  chain helpers,
                  | SegmentDecode, PageInfo,     |  .ini parser
                  | TextRegionDec kernel, ...    |
                  +------------------------------+
                              |
          +-------------------+--------------------+
          v                   v                    v
   Jb2_MQLapper(.cpp/.h)  Jb2_T4T6Lapper(...)   codsub.cpp/decsub.cpp
   MQ int/image coders    JBIG2 Huffman tables  T.4/T.6 MH/MR/MMR
          |                   |                    |
          v                   +---- StreamChain_s consumers ----+
   MQ_codec.cpp (Annex E)
          |
          v
   ImageUtil.cpp (bitmap & stream I/O)
```

### 5.2 Key data structures (from `ImageUtil.h` / `Jb2Common.h`)

- `Image_s` -- 2-D image. Holds `data`, geometry (`tbx0..tby1`, `width`,
  `height`, `col1step`, `row1step`), element `type`
  (`BIT1`, `CHAR`, `BYTE2`, `BYTE4`, ...), and `MaxValue`.
- `ImageChain_s` / `Jb2_ImageChain_s` -- linked lists used to carry a page's
  regions / a symbol dictionary's glyphs; traversal helpers
  (`ImageChainParentSearch`, `ImageChainChildSearch`, `ImageChainCreate`,
  `Jb2_ImageChainSearch`) are in `JBIG2Common.cpp`.
- `StreamChain_s` -- the read/write bit-stream buffer. Streams can be
  `JBIG2`, `JPEG`, `JPEG2000`, `JPEGXR`, `JPEG_XT`, `NoDiscard`; JBIG2
  streams use `NoDiscard` (no byte-stuffing). Operations live in
  `ImageUtil.cpp`: `StreamChainMake`, `StreamBitWrite`, `Stream{1,2,4}ByteWrite`,
  `StreamChainBind`, `StreamToFile`, `StreamChainDestory`, etc.
- `Jbig2Parameter_s` -- central encoder state; collects one pointer per
  segment type (`PageInfo`, `EndPage`, `SymbolDic`, `TextRegion`,
  `PatternDic`, `HalfRegion`, `GenRegion`) plus the image chains
  (`ImagePage`, `ImageTxt`, `ImageSym`, `ImagePat`, `ImageHaf`, `ImageGen`).
- `Jb2SegmentHeader_s` -- serialised segment-header fields (number, type,
  page association, referred-to segments, data length).
- Per-segment parameter structs: `SymbolDictionarySegment_s`,
  `TextRegionSegment_s`, `PatternDictionarySegment_s`,
  `HalftoneRegionSegment_s`, `GenericRegionSegment_s`,
  `PageInformationSegment_s`, `EndOfPageSegment_s`,
  `ColourPaletteSegment_s` -- each tracks the full set of flags and
  adaptive-template pixels specified in 7.4 of the spec (e.g. the generic
  region struct carries 12 `ATX`/`ATY` pairs for AMD2 extended templates).
- `mqcodec_s` -- MQ arithmetic coder state: `Creg`, `Areg`, `ctreg`,
  `B_buf`, `first_flag`, `numCX`, and `CX` + `index` arrays indexed by
  the JBIG2 context IDs (`IAAI`, `IADH`, `IADW`, ..., declared in
  `Jb2Common.h`; total slots = `Number_CX = 0x12000`).

### 5.3 Encoder flow (`Jbig2ENC.cpp`)

1. Write the 8-byte magic + flags byte + 4-byte page count.
2. `CreateHuffmanTable(ENC)` builds the 15 static Huffman tables A-O.
3. Allocate the MQ coder (`numCX = Number_CX = 0x12000`).
4. `SegmentCreate(Jb2Param)` materialises a `Jb2SegmentHeader_s[]` in the
   order: PageInformation -> user segments (Symbol Dictionaries -> Text
   Regions -> Generic Regions -> Halftone/Pattern) -> EndOfPage.
5. `SegmentEncode` iterates over the segment array and dispatches on
   `SegmentType`:
   - `SYMBOL_DICTIONARY` -> `SymbolDictionarySegmentEnc`.
   - `IMMEDIATE_TEXT_REGION` -> `TextRegionSegmentEnc`.
   - `IMMEDIATE_GENERIC_REGION` -> `ImmediateLosslessGenericRegionSegmentEnc`.
   - `PAGE_INFORMATION` -> `PageInformationSegmentEnc`.
   - `END_OF_PAGE` -> `EndOfPageSegmentEnc`.
   (`PATTERN_DICTIONARY`, halftone, and refinement branches are stubbed out
   in this reference implementation -- they are explicitly left `break`ing.)
6. Every segment body is written first, its length is patched back into
   the already-written 32-bit `SegmentDataLength` field.

### 5.4 Decoder flow (`Jbig2DEC.cpp`)

1. `StreamChainMake` loads the `.jb2` file into memory.
2. `JBIG2_DecMain` validates the magic, reads the number of pages,
   counts segments, and calls `SegmentDecode`.
3. `SegmentDecode` parses each segment header (7.2) and dispatches on
   segment type, updating a `Jb2_ImageChain_s *ImageSym` as symbol
   dictionaries are read, and accumulating decoded regions into an
   `ImageChain_s` that represents a composed page.
4. After `END_OF_PAGE`, `ImageChainParentSearch` walks back to the first
   page, and `JBIG2_Main` saves each decoded page bitmap as
   `<stem>NN.bmp` / `<stem>NN.tif`.

### 5.5 MQ arithmetic coder (`MQ_codec.cpp`)

- Implements ITU-T T.88 Annex E exactly, using the 47-state Qe probability
  table `QeIndexTable[47]` (identical to the JPEG / JBIG MQ definition).
- `InitMQ_Codec(codec, str, numCX, enc_or_dec, Eaddr, KindOfCode)`
  with `KindOfCode = JBIG2` disables J2K-style byte stuffing.
- Low-level primitives: `Enc_MQ`, `Dec_MQ`, `MQ_ByteIn/Out`,
  `MQ_DecRenorm`, `MQ_setbits`, `MQ_flush`.
- The higher-level wrappers in `Jb2_MQLapper.cpp` implement the
  **arithmetic integer decoding procedure** (Annex A): `MQ_DecInteger`,
  `MQ_EncInteger`, `MQ_DecIntegerIAID`, `MQ_EncIntegerIAID`,
  plus the **generic-region image** codec `MQ_EncImage` / `MQ_DecImage`
  that supports both the 4 standard templates (0, 1, 2, 3) and the AMD2
  **extended** template with up to 12 adaptive pixels (`ExtTemplate=1`).
  Refinement images are handled by `MQ_RefinementEncImage` /
  `MQ_RefinementDecImage`.

### 5.6 Huffman support (`Jb2_T4T6Lapper.cpp` + header)

- Static arrays `Huffman_Table_A..O` store each table as four parallel
  rows: `{code, code_length, range_width_bits, value_start, kind}`.
  Kind 0 = direct, 2 = special range (negative ranges).
- `CreateHuffmanTable(enc_or_dec)` fills a `Jb2HuffmanTable_s` array; the
  encoder and decoder use the same static data via different views
  (`EncC/EncC_L/...` vs. `DecC/DecC_L/...`).
- `JBIG2_HuffEnc(val, str, table)` and `JBIG2_HuffDec(str, table)` are
  the per-value entry points. `JBIG2_ID_Dec` reads a short ID prefix for
  the "SymbolID" Huffman mode.

### 5.7 T.4/T.6 (CCITT Group 3/4) codec

`codsub.cpp` and `decsub.cpp` implement **MH (Modified Huffman)**,
**MR (Modified READ, 2-D)**, and **T.6 (MMR)** line coding, used by JBIG2
for "MMR" generic region segments (where the `MMR` flag in
`GenericRegionSegment_s` is set). The tables live in `T4T6codec.h`:
`WhiteTerminateCode`, `BlackTerminateCode` (64+40 entries each, including
make-up codes for runs >= 64), and `ControlCode` (PASS, HORZ, VL3..VR3,
EOL, RTC). Entry points are `T4T6DecMain`, `T4T6Encmain`, `CodInit`,
`CodLine`, `CodEnd`, `DecMHLine`, `DecMRLine`, `DecLine`, `DFindEOL`,
`DCheckEOL`, etc.

### 5.8 T.44 / T.45 colour helpers (`T45_codec.cpp`)

Small wrappers used by `Jbig2ENC.cpp` / `Jbig2DEC.cpp` when a text region
carries the AMD3 colour extension (the `-txt -Param -ColorExt ...` tokens
in `jbig2_Param8.ini`): `T45_Enc` serialises the palette components into
the stream; `T45_Dec` reads them back. The naming reflects the source
material (ITU-T T.44 MRC / T.45 run-length colour) even though only a
simple palette is actually used by JBIG2.

### 5.9 `ImageUtil.cpp` -- image I/O

Handles:
- **BMP**: `LoadBmp` / `SaveBmp777` (1/8/24-bpp, row-aligned to 4 bytes).
- **TIFF**: `LoadTif` / `SaveTiff` with IFD entries enumerated in the
  huge `TagXXX` constant block at the top of `ImageUtil.h`.
- **PPM**: `LoadPpm` / `SavePpm`.
- **RAW** / **RAW_HDphoto**: `LoadRAW`, `LoadRAW_HDphoto`, `SaveRAW`
  (used by `imgcomp` for format `raw`).
- **Bit<->byte conversion**: `ImageBit1ToChar` (JBIG2 stores bitmaps
  unpacked as 1 byte/pixel internally) and `ImageCharToBit1`.
- **Stream primitives**: `StreamChainMake`, `StreamBitWrite` /
  `StreamBitWriteJ2K` / `StreamBitWriteJPG` / `StreamBitWriteJXR` (JBIG2
  uses the plain `NoDiscard` variant), `StreamToFile`,
  `StreamChainBind`, `StreamChainTruncate`, `Stream{1,2,4}ByteWrite`, etc.
- **Utility math**: `ceil2`, `floor2`, `floorlog2`, `ceil2log2`, `Umod`,
  `Rounding`, `fRounding`.

### 5.10 Debug hooks (`Jb2_Debug.cpp`, `Jb2_Debug.h`)

Flags `JBIG2_DEBUG00..06` control optional per-segment BMP dumps during
encode/decode. In the released tree, all are `0` except `JBIG2_DEBUG05`
("TextRegion Image output"), which is why the test run produces files
like `AggImage*.bmp`, `ImageResiEnc000.bmp`, `ImageResiDec00*.bmp` --
they are debug snapshots of text-region aggregation, not conformance
artefacts.

---

## 6. Conformance test data

Per K.1.3 of the spec, **`JBIG2_ConformanceData-A20180829/` is the master
copy** of the 10 official test vectors. The `test/` subdirectory of the
sample software contains the same reference `.bmp` and pre-made `.jb2`
files that the test Makefile needs locally, plus the per-test `.ini`
parameter files and the small symbol-dictionary BMPs (`Sym000.bmp` ...
`Sym100_*.bmp`).

The mapping (from `K.1.3 Table K.1`) is:

| # | Codestream              | Reference decoded BMP(s)                                                             | Source BMP          | Purpose (spec wording)                       | Target    |
|---|-------------------------|--------------------------------------------------------------------------------------|---------------------|----------------------------------------------|-----------|
| 1 | CodeStreamTest1_TT1.jb2 | codeStreamTest1_TT1_TT00.bmp, _TT01.bmp, _TT02.bmp                                   | codeStreamTest1.bmp / codeStreamTest2.bmp | Multi-Page & Huffman coding                  | mse = 0 |
| 2 | codeStreamTest1_TT2.jb2 | codeStreamTest1_TT2_TT00.bmp                                                          | codeStreamTest1.bmp | Huffman coding                               | mse = 0 |
| 3 | codeStreamTest1_TT3.jb2 | codeStreamTest1_TT3_TT00.bmp                                                          | codeStreamTest1.bmp | Arithmetic coding                            | mse = 0 |
| 4 | codeStreamTest1_TT4.jb2 | codeStreamTest1_TT4_TT00.bmp                                                          | codeStreamTest1.bmp | Generic Region Template = 1                  | mse = 0 |
| 5 | codeStreamTest1_TT5.jb2 | codeStreamTest1_TT5_TT00.bmp                                                          | codeStreamTest1.bmp | Symbol Dictionary Segment refinement         | mse = 0 |
| 6 | codeStreamTest2_TT6.jb2 | codeStreamTest2_TT6_TT00.bmp                                                          | codeStreamTest2.bmp | Text Region Segment refinement               | mse = 0 |
| 7 | codeStreamTest1_TT7.jb2 | codeStreamTest1_TT7_TT00.bmp                                                          | codeStreamTest1.bmp | Generic Region using expand (AMD2) context   | mse = 0 |
| 8 | codeStreamTest3_TT8.jb2 | codeStreamTest3_TT8_TT00.bmp                                                          | codeStreamTest3.bmp | Text Region using pallet colour (AMD3)       | mse = 0 |
| 9 | F01_200_TT9.jb2         | F01_200_TT9_TT00.bmp                                                                  | F01_200.bmp         | All generic region using Huffman             | mse = 0 |
|10 | F01_200_TT10.jb2        | F01_200_TT10_TT00.bmp                                                                 | F01_200.bmp         | All generic region using Arithmetic          | mse = 0 |

For each row the conformance requirement is the same: re-encoding the
source bitmap with the matching `.ini` (or no `.ini` for T10), re-decoding
the resulting codestream, and comparing against the reference decoded BMP
must yield `mse = 0`.

---

## 7. Relationship to the surrounding repository

This directory sits at `vendor/T-REC-T.88-201808/` inside a larger Rust
workspace (`jbig2-rust/`). The reference software here is **not** built as
part of the main Rust crate -- it is kept verbatim as:

- A **ground-truth reference implementation** of the JBIG2 bitstream
  syntax and arithmetic/Huffman coders.
- A **conformance test suite** that any Rust port can point at by feeding
  the bundled `.jb2` files into its decoder and checking
  `mse(decoded, F01_200_TT9_TT00.bmp) == 0`, etc.
- The **authoritative English-language specification** (`ITU-T_T_88__08_2018.docx`).

The repo convention of using a local `CARGO_HOME` (e.g.
`CARGO_HOME=./.cargo`) for Cargo commands applies to the Rust side of
the tree; it is not relevant to the `make`-based build of this
reference package.

---

## 8. Quick reference cheat-sheet

```bash
# Build & run all 10 conformance tests
cd vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829
make                  # builds jbig2 + imgcomp + runs T1..T10

# Decode any bundled codestream
source/jbig2 -i test/codeStreamTest1_TT1 -f jb2 -o /tmp/page -F bmp
#   -> /tmp/page00.bmp, /tmp/page01.bmp, /tmp/page02.bmp

# Encode a bitmap with default generic-region arithmetic coding
source/jbig2 -i test/F01_200 -f bmp -o /tmp/F01 -F jb2

# Encode with a specific feature profile
source/jbig2 -i test/codeStreamTest1 -f bmp -o /tmp/tt3 -F jb2 \
             -ini test/jbig2_Param3.ini

# Compare two bitmaps (mse = 0 means pixel-exact)
source/imgcomp -t test/F01_200           -f bmp \
               -T test/F01_200_TT10_TT00 -F bmp -m mse

# Run MQ / Huffman / refinement internal self-tests
source/jb2z1
```
