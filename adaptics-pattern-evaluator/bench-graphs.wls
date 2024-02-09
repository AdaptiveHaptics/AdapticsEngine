#!/usr/bin/env wolframscript
(* ::Package:: *)

(* ::Title:: *)
(*Create PDF Figure*)


iqrc=9.15


countOutliers[d_]:=Module[{fences,outliers},
fences = Quartiles[d][[{1, 3}]] + {-1, 1} iqrc InterquartileRange[d];
outliers = Select[Not @* Between[fences]] @ d;
Length[outliers]
]


benchresults = Import[FileNameJoin[{NotebookDirectory[], "benchresults.csv"}], "Dataset", HeaderLines -> {0, 1}];
benchresults = (Quantity[#, "Seconds"] & /@ #) & /@ benchresults;
benchresults["base"][1]
benchresults = (UnitConvert[1/#, "Kilohertz"] & /@ #) & /@ benchresults;
benchresults["base"][1]
benchresults = KeyMap[# /. {
      "base" -> "Baseline (1 KF)",
       "rainbench" -> "RainBench (62 KFs)",
      "rainbench2x" -> "RainBench2x (124 KFs)",
      "rainbenchmoreformulas" -> "RainBenchF (~9x Formulas)"
      } &,  Normal[benchresults]];

statFns=Association@((ToString[#]->#)& /@ {Min,Quartiles,Max,StandardDeviation,Length,countOutliers})
Dataset[(r|->(#[r])&/@ statFns)/@benchresults]


customTextStyle[content_, size_] :=
  Style[content, FontSize -> size, FontFamily -> "Helvetica Neue"]

bc=BoxWhiskerChart[
	Reverse[benchresults],
	"Outliers",
	ChartElementFunction->ChartElementDataFunction["BoxWhisker","IQRCoefficient"->iqrc],
	BarOrigin->Left,
	BarSpacing->Small,
	ChartLabels->Placed[Automatic,After],
	(*FrameLabel->{None,"Benchmarks"},*)
	Frame->False,Axes->True,AxesLabel->{"kHz",""},
	PlotRange->{{0,1500},Automatic},AxesOrigin->{0,Automatic},
	Ticks->{{#,""}&/@Range[1,4],{#, Style[ToString[#],FontSize -> 11,FontFamily -> "Helvetica Neue"]} & /@{40,500,1000,1500}},
	LabelStyle -> {FontFamily -> "Helvetica Neue",FontSize->12},
	Prolog->{
		Text[customTextStyle["Benchmarks",15],{-55, (Length[benchresults]+1)/2},Automatic,{0,1}], (* this looks different in the latex *)
		Text[customTextStyle["Max Engine Update Rate (higher is better)",13.25],{1500*0.56,-0.58}],
		RGBColor[52/85, 59/85, 28/51],AbsoluteThickness[2.5],HalfLine[{{40,0},{40,1}}],
		Text[customTextStyle["Device Update Rate",13.5],{40+20, 4.8},{-1,0}]
	},
	Background->White,
	ChartStyle->RGBColor[151/255, 11/17, 184/255],
	PlotRange->All,
	ImagePadding -> {{5,75},{25,5}},
	ImageSize->470
]


exportpath=FileNameJoin[{NotebookDirectory[], "engine-bench-boxchart.pdf"}];
Export[exportpath, bc,ImageSize->480]
Show[Import[exportpath,RasterSize->800][[1]],ImageSize->300]


(* ::Title:: *)
(*Old Charts*)


bcv=BoxWhiskerChart[
	benchresults,
	"Outliers",
	ChartLabels->Automatic,
	Frame->False,Axes->True,AxesLabel->Automatic,
	PlotRange->{0,1500},AxesOrigin->{Automatic,0},
	Ticks->{Automatic,{40,500,1000,1500}},
	GridLines->{None,{{40,Directive[Green,AbsoluteThickness[4]]}}},
	Epilog->{Text[Style["Device Update Rate",Green,Bold,FontSize->11.5],{0.5,40+40}]},
	ImageSize->600,
	AspectRatio->0.45
]


DistributionChart[benchresults,ChartLabels->Automatic,AxesLabel->Automatic,ChartElementFunction->"HistogramDensity"]


Around[Mean[#],StandardDeviation[#]]&/@benchresults
BarChart[%,ChartLabels->Automatic,AxesLabel->Automatic]
