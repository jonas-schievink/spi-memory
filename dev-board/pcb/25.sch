EESchema Schematic File Version 4
LIBS:dev-board-cache
EELAYER 30 0
EELAYER END
$Descr A4 11693 8268
encoding utf-8
Sheet 2 17
Title ""
Date ""
Rev ""
Comp ""
Comment1 ""
Comment2 ""
Comment3 ""
Comment4 ""
$EndDescr
$Comp
L Memory_EEPROM:25LCxxx U5
U 1 1 5E08F69E
P 5150 3950
AR Path="/5E08F0E6/5E08F69E" Ref="U5"  Part="1" 
AR Path="/5E0DF126/5E08F69E" Ref="U6"  Part="1" 
AR Path="/5E0EA7AF/5E08F69E" Ref="U7"  Part="1" 
AR Path="/5E0EA7B4/5E08F69E" Ref="U8"  Part="1" 
AR Path="/5E121445/5E08F69E" Ref="U9"  Part="1" 
AR Path="/5E12144A/5E08F69E" Ref="U10"  Part="1" 
AR Path="/5E12144F/5E08F69E" Ref="U11"  Part="1" 
AR Path="/5E121454/5E08F69E" Ref="U12"  Part="1" 
AR Path="/5E6B272D/5E08F69E" Ref="U?"  Part="1" 
AR Path="/5E6B2F93/5E08F69E" Ref="U6"  Part="1" 
AR Path="/5E6CD70C/5E08F69E" Ref="U7"  Part="1" 
AR Path="/5E6DA750/5E08F69E" Ref="U8"  Part="1" 
AR Path="/5E6E82DE/5E08F69E" Ref="U9"  Part="1" 
AR Path="/5E6F68FB/5E08F69E" Ref="U10"  Part="1" 
AR Path="/5E7061AD/5E08F69E" Ref="U11"  Part="1" 
AR Path="/5E71622F/5E08F69E" Ref="U12"  Part="1" 
AR Path="/5D73BCFF/5E08F69E" Ref="U20"  Part="1" 
AR Path="/5D73BD13/5E08F69E" Ref="U19"  Part="1" 
AR Path="/5D73BD27/5E08F69E" Ref="U18"  Part="1" 
AR Path="/5D73BD3B/5E08F69E" Ref="U17"  Part="1" 
AR Path="/5D73BD4F/5E08F69E" Ref="U16"  Part="1" 
AR Path="/5D73BD63/5E08F69E" Ref="U15"  Part="1" 
AR Path="/5D73BD77/5E08F69E" Ref="U14"  Part="1" 
AR Path="/5D73BD8B/5E08F69E" Ref="U13"  Part="1" 
F 0 "U20" H 4900 4200 50  0000 C CNN
F 1 "25LCxxx" H 5350 4200 50  0000 C CNN
F 2 "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm" H 5150 3950 50  0001 C CNN
F 3 "http://ww1.microchip.com/downloads/en/DeviceDoc/21832H.pdf" H 5150 3950 50  0001 C CNN
	1    5150 3950
	1    0    0    -1  
$EndComp
Wire Wire Line
	5150 3650 4750 3650
Wire Wire Line
	4750 3650 4750 3850
Wire Wire Line
	4750 3850 4750 3950
Connection ~ 4750 3850
Text HLabel 4750 4050 0    50   Input ~ 0
~CS
Connection ~ 4750 3650
$Comp
L Device:C C7
U 1 1 5E0FC1AC
P 4200 3950
AR Path="/5E0DF126/5E0FC1AC" Ref="C7"  Part="1" 
AR Path="/5E08F0E6/5E0FC1AC" Ref="C6"  Part="1" 
AR Path="/5E0EA7AF/5E0FC1AC" Ref="C8"  Part="1" 
AR Path="/5E0EA7B4/5E0FC1AC" Ref="C9"  Part="1" 
AR Path="/5E121445/5E0FC1AC" Ref="C10"  Part="1" 
AR Path="/5E12144A/5E0FC1AC" Ref="C11"  Part="1" 
AR Path="/5E12144F/5E0FC1AC" Ref="C12"  Part="1" 
AR Path="/5E121454/5E0FC1AC" Ref="C13"  Part="1" 
AR Path="/5E6B272D/5E0FC1AC" Ref="C?"  Part="1" 
AR Path="/5E6B2F93/5E0FC1AC" Ref="C7"  Part="1" 
AR Path="/5E6CD70C/5E0FC1AC" Ref="C8"  Part="1" 
AR Path="/5E6DA750/5E0FC1AC" Ref="C9"  Part="1" 
AR Path="/5E6E82DE/5E0FC1AC" Ref="C10"  Part="1" 
AR Path="/5E6F68FB/5E0FC1AC" Ref="C11"  Part="1" 
AR Path="/5E7061AD/5E0FC1AC" Ref="C12"  Part="1" 
AR Path="/5E71622F/5E0FC1AC" Ref="C13"  Part="1" 
AR Path="/5D73BCFF/5E0FC1AC" Ref="C23"  Part="1" 
AR Path="/5D73BD13/5E0FC1AC" Ref="C22"  Part="1" 
AR Path="/5D73BD27/5E0FC1AC" Ref="C21"  Part="1" 
AR Path="/5D73BD3B/5E0FC1AC" Ref="C20"  Part="1" 
AR Path="/5D73BD4F/5E0FC1AC" Ref="C19"  Part="1" 
AR Path="/5D73BD63/5E0FC1AC" Ref="C18"  Part="1" 
AR Path="/5D73BD77/5E0FC1AC" Ref="C17"  Part="1" 
AR Path="/5D73BD8B/5E0FC1AC" Ref="C16"  Part="1" 
F 0 "C23" H 4315 3996 50  0000 L CNN
F 1 "100n" H 4315 3905 50  0000 L CNN
F 2 "Capacitor_SMD:C_0805_2012Metric_Pad1.15x1.40mm_HandSolder" H 4238 3800 50  0001 C CNN
F 3 "~" H 4200 3950 50  0001 C CNN
	1    4200 3950
	1    0    0    -1  
$EndComp
Wire Wire Line
	4200 3650 4200 3800
Wire Wire Line
	4200 3650 4750 3650
Wire Wire Line
	4200 4100 4200 4250
Wire Wire Line
	4200 4250 5150 4250
Connection ~ 5150 4250
Wire Wire Line
	5150 4250 5150 4350
Text HLabel 5550 3850 2    50   Input ~ 0
SCLK
Text HLabel 5550 3950 2    50   Input ~ 0
MOSI
Text HLabel 5550 4050 2    50   Output ~ 0
MISO
Text HLabel 5150 4350 3    50   UnSpc ~ 0
GND
Text HLabel 5150 3550 1    50   UnSpc ~ 0
VCC
Wire Wire Line
	5150 3550 5150 3650
Connection ~ 5150 3650
$EndSCHEMATC
