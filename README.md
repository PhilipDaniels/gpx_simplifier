# gapix

GaPiX: GPX analysis and information

A small command-line tool to join and simplify GPX tracks.

I wrote this tool because the GPX files produced by my Garmin
Edge 1040 are huge - about 13MB for a 200km ride. This is far
too large for [Audax UK](https://www.audax.uk/) to validate
for a DIY ride (max file size of 1.25Mb). The files are so
large because the Edge 1040 writes a trackpoint every second, each
one has extra information such as heart rate and temperature, and it
records lat-long to a ridiculous number of decimal places,
e.g. "53.0758009292185306549072265625" and elevation likewise
to femtometre precision "173.8000030517578125".

In reality, the device only measures elevation to 1 decimal place and
6 decimal places are sufficient to record lat-long to within 11cm
of accuracy: see https://en.wikipedia.org/wiki/Decimal_degrees

This program shrinks the files down by simplifying the individual
trackpoints to just lat-long, elevation and time and optionally
by applying the [Ramer-Douglas-Peucker algorithm](https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm) to
eliminate unnecessary trackpoints - that is, those that lie
along the line.


# How to use

When **gapix** is run it looks for its input files
in the same folder as the exe. This is mainly for convenience -
I have a known folder containing a copy of the exe, I then
drop the GPXs I want to process into that folder and double-click
a batch file setup with the appropriate command line options
to process them. The program produces an output
filename ending in ".simplified.gpx" and never overwrites the
source file. If the output file already exists, nothing happens.

There are two command line options:

* `--metres=NN` - simplify the output file by applying the RDP
  algorithm with an accurancy of NN metres. 10 is a good value
  (see below for some estimates of reduction sizes).
* `--join` - joins all the input files together, producing
  one file with a single track. The name of the first file is
  used to derive the name of the output file.

If you specify both options then the input files will be joined
and then the result file simplified. But typically, I have
two folders setup with separate batch files, one for
joining and one for simplifying. For example, in my
"simplify" folder I have a batch file with the command

`gpx_simplifier.exe --metres=10`

which gives a very good size reduction while still being an
excellent fit to the road.


# Size Reduction Estimates

The original file is 11.5Mb with 31,358 trackpoints and was 200km long.

It was from a Garmin Edge 1040 which records 1 trackpoint every second. 
including a lot of extension data such as heartrate and temperature.

|--metres|Output Points|File Size|Quality|
|-|-|-|-|
|1  |4374 (13%) |563Kb|Near-perfect map to the road|
|5  |1484 (4.7%)|192Kb|Very close map to the road, mainly stays within the road lines|
|10 |978 (3.1%) |127Kb|Very Good - good enough for submission to Audax UK|
|20 |636 (2.0%) |83Kb |Ok - within a few metres of the road|
|50 |387 (1.2%) |51Kb |Poor - cuts off a lot of corners|
|100|236 (0.8%) |31Kb |Very poor - significant corner truncation|

# Installation

There is a release on Github, one for Windows and one for Linux.
Or build from source using cargo.

# Caveats
* Has only been tested on my own GPX files from a Garmin Edge 1040.


# TODO
- Full pass lossless read and write of the complete XSD.
- Pass some options into the gpx_writer to control the fidelity.
- Track splitting
- Waypoint processing
- Create an IndentingWriter based off the 'indent_write' crate.
- Reverse geocode the stopped stages and the first and last point.
- Use Rayon - CAN'T - Time crate blows up in to_local_offset.
- Change to use Chrono and Chrono-TZ? Probably. First need to be
  able to reverse geocode lat-lon to timezone name.

# Bugs
- I think it's technically wrong to simply merge all tracks and segments?
  They may exist due to GPS interruptions, device restarts etc.
  Fixing this would make things a lot more complicated though.
