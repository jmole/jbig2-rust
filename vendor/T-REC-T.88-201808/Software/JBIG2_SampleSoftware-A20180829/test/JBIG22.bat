rem del *.img
rem del *.bmp
rem del *.jb2
rem del *.csv
rem del *.tif
pause

jbig2 -i codeStreamTest1_TT1 -f jb2 -o codeStreamTest1_TT1_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT1_TT00 -F bmp -m mse
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT1_TT01 -F bmp -m mse
imgcomp -t codeStreamTest2 -f bmp -T codeStreamTest1_TT1_TT02 -F bmp -m mse
pause

:Param2
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT2 -F jb2 -ini jbig2_Param2.ini
jbig2 -i codeStreamTest1_TT2 -f jb2 -o codeStreamTest1_TT2_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT2_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param3
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT3 -F jb2 -ini jbig2_Param3.ini
jbig2 -i codeStreamTest1_TT3 -f jb2 -o codeStreamTest1_TT3_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT3_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param4
rem Generic Region Template=1 test
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT4 -F jb2 -ini jbig2_Param4.ini
jbig2 -i codeStreamTest1_TT4 -f jb2 -o codeStreamTest1_TT4_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT4_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param5
rem SymbolDictionary Refinement symbol define test
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT5 -F jb2 -ini jbig2_Param5.ini
jbig2 -i codeStreamTest1_TT5 -f jb2 -o codeStreamTest1_TT5_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT5_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param6
rem TextRegionSegment Symbol Refinement set test 
jbig2 -i codeStreamTest2 -f bmp -o codeStreamTest2_TT6 -F jb2 -ini jbig2_Param6.ini
jbig2 -i codeStreamTest2_TT6 -f jb2 -o codeStreamTest2_TT6_TT -F bmp
imgcomp -t codeStreamTest2 -f bmp -T codeStreamTest2_TT6_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param7
rem AMD2 test
jbig2 -i codeStreamTest1 -f bmp -o codeStreamTest1_TT7 -F jb2 -ini jbig2_Param7.ini
jbig2 -i codeStreamTest1_TT7 -f jb2 -o codeStreamTest1_TT7_TT -F bmp
imgcomp -t codeStreamTest1 -f bmp -T codeStreamTest1_TT7_TT00 -F bmp -m mse
pause
goto Param8

:Param8
rem AMD3 test
jbig2 -i codeStreamTest3 -f bmp -o codeStreamTest3_TT8 -F jb2 -ini jbig2_Param8.ini
jbig2 -i codeStreamTest3_TT8 -f jb2 -o codeStreamTest3_TT8_TT -F bmp
imgcomp -t codeStreamTest3 -f bmp -T codeStreamTest3_TT8_TT00 -F bmp -m mse
pause
rem goto ENDEND

:Param9
jbig2 -i F01_200 -f bmp -o F01_200_TT9 -F jb2 -ini jbig2_Param9.ini
jbig2 -i F01_200_TT9 -f jb2 -o F01_200_TT9_TT -F bmp
imgcomp -t F01_200 -f bmp -T F01_200_TT9_TT00 -F bmp -m mse
pause

:Param10
jbig2 -i F01_200 -f bmp -o F01_200_TT10 -F jb2
jbig2 -i F01_200_TT10 -f jb2 -o F01_200_TT10_TT -F bmp
imgcomp -t F01_200 -f bmp -T F01_200_TT10_TT00 -F bmp -m mse
pause

:ENDEND
