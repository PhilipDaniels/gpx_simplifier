# gpx_simplifier

The GPS files produced by the Garmin Edge 1040 are huge -
about 13MB for a 200km ride. This program shrinks them down,
the main aim being to produce a file that can be uploaded
to Audax UK to validate a DIY ride.

It makes the following changes

* Removes irrelevant nodes such as temperature, elevation
  and heart rate
* Writes only the first 6 decimal places of accuracy in
  the "lat" and "lon" attributes of the "<trkpt>" node.
  6 d.p. are sufficient to locate a point to within 11cm
  of accuracy: see https://en.wikipedia.org/wiki/Decimal_degrees


# Running

Put the gpx_simplifier.exe into a folder.

Copy any GPXes you want to simplify into the same folder.

Run the EXE (double click is OK on windows).

The program looks for all the GPX files in the same directory
as the EXE and writes a new ".simplified.gpx" file in the same
directory. No input files are changed.
