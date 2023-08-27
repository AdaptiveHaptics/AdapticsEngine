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
	Ticks->{{#,""}&/@Range[1,4],{40,500,1000,1500}},
	Prolog->{
		Text[Style["Benchmarks",Bold,FontSize->12],{-50, (Length[benchresults]+1)/2},Automatic,{0,1}],
		Text[Style["Max Update Rate (higher is better)",Bold,FontSize->11],{1500*0.56,-0.55}],
		RGBColor[52/85, 59/85, 28/51],AbsoluteThickness[2.5],HalfLine[{{40,0},{40,1}}],
		Text[Style["Device Update Rate",Bold,FontSize->11.5],{40+20, 4.8},{-1,0}]
	},
	Background->White,
	ChartStyle->RGBColor[151/255, 11/17, 184/255],
	PlotRange->All,
	ImagePadding -> {{5,65},{25,5}},
	ImageSize->450
]


Export[FileNameJoin[{NotebookDirectory[], "engine-bench-boxchart.pdf"}], bc]


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









