use args::parse_args;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};
use geo::{coord, point, GeodesicDistance, LineString, SimplifyIdx};
use model::{Gpx, MergedGpx, Stop, TrackPoint};
use quick_xml::reader::Reader;
use time::{format_description, Duration, OffsetDateTime, UtcOffset};
use std::collections::HashSet;
use std::io::Write;
use std::{
    fs::{read_dir, File},
    io::BufWriter,
    path::{Path, PathBuf},
};
use time::format_description::well_known::Rfc3339;

mod args;
mod model;
mod section;

fn main() {
    let args = parse_args();

    let exe_dir = get_exe_dir();
    let input_files = get_list_of_input_files(&exe_dir);
    if input_files.is_empty() {
        println!("No .gpx files found");
        return;
    }

    // We can operate in 3 modes depending on the command line
    // arguments.
    // --metres=NN          - simplify each input file individually
    // --join               - join all the input files into a single file
    // --join --metres=NN   - join into a single file then simplify

    // Read all files into RAM.
    let gpxs: Vec<Gpx> = input_files.iter().map(|f| read_gpx_file(f)).collect();

    // Within each file, merge multiple tracks and segments into a single
    // track-segment.
    let mut gpxs: Vec<MergedGpx> = gpxs.iter().map(|f| f.merge_all_tracks()).collect();

    // Join if necessary. Keep as a vec (of one element) so that
    // following loop can be used whether we join or not.
    if args.join {
        gpxs = vec![join_input_files(gpxs)];
    }

    // Skip any files if the output already exists. It's wasteful to do this
    // after the load and parse and join, but it keeps the logic simpler.
    gpxs.retain(|gpx| {
        let output_filename = make_simplified_filename(&gpx.filename);
        if output_filename.exists() {
            println!(
                "Skipping {:?} because the output file already exists",
                &gpx.filename
            );
            false
        } else {
            true
        }
    });

    if args.detect_stops {
        for gpx in &mut gpxs {
            calculate_distance_and_speed(&mut gpx.points);
            let stops = detect_stops(&gpx.points, args.resume_speed, args.min_stop_time);
            let mut io = std::io::stdout().lock();
            write_stop_report(&mut io, &gpx, &stops);

            let p = make_stats_filename(&gpx.filename);
            let mut writer = BufWriter::new(File::create(&p).unwrap());
            write_stop_report(&mut writer, &gpx, &stops);
        }
    }

    // Simplify if necessary.
    if let Some(metres) = args.metres {
        let epsilon = metres_to_epsilon(metres);

        for merged_gpx in &mut gpxs {
            let start_count = merged_gpx.points.len();
            reduce_trackpoints_by_rdp(&mut merged_gpx.points, epsilon);
            println!(
                "Using Ramer-Douglas-Peucker with a precision of {metres}m (epsilon={epsilon}) reduced the trackpoint count from {start_count} to {} for {:?}",
                merged_gpx.points.len(),
                merged_gpx.filename
            );
        }
    }

    for merged_gpx in gpxs {
        let output_filename = make_simplified_filename(&merged_gpx.filename);
        write_output_file(&output_filename, &merged_gpx);
    }
}

/// Calculates the distance from one trackpoint to the next using the geo
/// crate. This seems to be reasonably accurate, but over-estimates the
/// total distance by approx 0.5% compared to plotaroute.com.
fn calculate_distance_and_speed(points: &mut [TrackPoint]) {
    if points.len() < 2 {
        return;
    }

    // Distances first.
    // n.b. x=lon, y=lat. If you do it the other way round the
    // distances are wrong - a lot wrong.
    let mut cum_distance = 0.0;
    let mut p1 = point!(x: points[0].lon as f64, y: points[0].lat as f64);
    for i in 1..points.len() {
        let p2 = point!(x: points[i].lon as f64, y: points[i].lat as f64);
        let distance = p1.geodesic_distance(&p2);
        cum_distance += distance;
        points[i].distance_from_prev_metres = distance as f32;
        points[i].cumulative_distance_metres = cum_distance as f32;
        p1 = p2;
    }

    // Then speed is easy. We can calculate this for every point but the first.
    // Again, this is heavily dependent upon the accuracy of the distance
    // calculation, but seems "about right".
    // TODO: Probably would be better with smoothing.
    for i in 1..points.len() {
        let time_delta_seconds = (points[i].time - points[i - 1].time).as_seconds_f32();
        points[i].speed_kmh = calc_speed_kmh(points[i].distance_from_prev_metres, time_delta_seconds);

        // While we are at it, also fill in the climb figures.
        // TODO: Doesn't work, probably not enough change per point.
        points[i].ascent_from_prev_metres = points[i].ele - points[i - 1].ele;
        if points[i].ascent_from_prev_metres > 0.0 {
            points[i].cumulative_ascent_metres = points[i - 1].cumulative_ascent_metres + points[i].ascent_from_prev_metres;
        } else {
            points[i].cumulative_descent_metres = points[i - 1].cumulative_descent_metres + points[i].ascent_from_prev_metres.abs();
        }
    }
}

/// You are determined to be stopped if your speed drops below MIN_SPEED km/h and does not
/// go above 'resume_speed' until at least 'min_stop_time' minutes have passed.
fn detect_stops(points: &[TrackPoint], resume_speed: u8, min_stop_time: u8) -> Vec<Stop> {
    const MIN_SPEED: f32 = 0.1;
    let resume_speed = resume_speed as f32;
    let min_stop_time = min_stop_time as f32 * 60.0; // convert to seconds

    let mut iter = points.iter().enumerate();

    // Skip the first point, it always has speed 0.
    iter.next();

    let mut stops = Vec::new();

    while let Some((start_idx, start_point)) = iter.find(|(_, p)| p.speed_kmh < MIN_SPEED) {
        // Find the next point that has a speed of at least resume_speed, i.e. we started riding again.
        if let Some((end_idx, end_point)) = iter.find(|(_, p)| p.speed_kmh > resume_speed) {
            if (end_point.time - start_point.time).as_seconds_f32() > min_stop_time {
                let stop = Stop {
                    start: start_point.clone(),
                    start_idx,
                    end: end_point.clone(),
                    end_idx
                };
    
                stops.push(stop);
            }
        }
    }

    stops
}

fn calc_speed_kmh(metres: f32, seconds: f32) -> f32 {
    (metres / seconds) * 3.6
}

fn write_stop_report<W: Write>(w: &mut W, gpx: &MergedGpx, stops: &[Stop]) {
    let stopped_time: Duration = stops.iter().map(|s| s.duration()).sum();
    let moving_time = gpx.total_time() - stopped_time;
    let min_ele = gpx.min_elevation();
    let max_ele = gpx.max_elevation();

    writeln!(w, "Distance     : {:.2} km", gpx.distance_km()).unwrap();
    writeln!(w, "Start time   : {}", format_utc_date(gpx.start_time())).unwrap();
    writeln!(w, "End time     : {}", format_utc_date(gpx.end_time())).unwrap();
    writeln!(w, "Total time   : {}", gpx.total_time()).unwrap();
    writeln!(w, "Moving time  : {}", moving_time).unwrap();
    writeln!(w, "Stopped time : {}", stopped_time).unwrap();
    writeln!(w, "Moving speed : {:.2} km/h", calc_speed_kmh(gpx.distance_metres(), moving_time.as_seconds_f32())).unwrap();
    writeln!(w, "Overall speed: {:.2} km/h", calc_speed_kmh(gpx.distance_metres(), gpx.total_time().as_seconds_f32())).unwrap();
    writeln!(w, "Total ascent : {:.2} m", gpx.total_ascent_metres()).unwrap();
    writeln!(w, "Total descent: {:.2} m", gpx.total_descent_metres()).unwrap();
    writeln!(w, "Min elevation: {} m, at {:.2} km, {}",
        min_ele.ele, min_ele.cumulative_distance_metres / 1000.0, format_utc_date(min_ele.time)
        ).unwrap();
    writeln!(w, "Max elevation: {} m, at {:.2} km, {}",
        max_ele.ele, max_ele.cumulative_distance_metres / 1000.0, format_utc_date(max_ele.time)
        ).unwrap();
    writeln!(w).unwrap();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Stop", "Start", "End", "Length", "Location"])
        .set_content_arrangement(ContentArrangement::Dynamic);
        
    for (idx, stop) in stops.iter().enumerate() {
        table.add_row(vec![
            Cell::new(idx + 1).set_alignment(CellAlignment::Right),
            Cell::new(format_utc_and_local_date(stop.start.time, "\n")),
            Cell::new(format_utc_and_local_date(stop.end.time, "\n")),
            Cell::new(stop.duration()),
            Cell::new(format!("{}\n({},{})", "unk", stop.start.lat, stop.start.lon)),
        ]);
    }

    writeln!(w, "{}", table).unwrap();
}

fn make_simplified_filename(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    p.set_extension("simplified.gpx");
    p
}

fn make_stats_filename(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    p.set_extension("stats.txt");
    p
}

/// TODO: This is awful, does a clone of the first element.
fn join_input_files(mut input_files: Vec<MergedGpx>) -> MergedGpx {
    let required_capacity: usize = input_files.iter().map(|f| f.points.len()).sum();
    let mut m = input_files[0].clone();
    m.points = Vec::with_capacity(required_capacity);

    for f in &mut input_files {
        println!("Joining {:?}", f.filename);
        m.points.append(&mut f.points);
    }

    println!("Joined {} files", input_files.len());

    m
}

/// We take input from the user in "metres of accuracy".
/// The 'geo' implementation of RDP requires an epsilon
/// which is relative to the coordinate scale in use.
/// Since we are using lat-lon, we need to convert metres
/// using the following relation: 1 degree of latitude = 111,111 metres
fn metres_to_epsilon(metres: u16) -> f32 {
    metres as f32 / 111111.0
}

/// Feed the points into the GEO crate so we can use its implementation
/// of https://en.wikipedia.org/wiki/Ramer%E2%80%93Douglas%E2%80%93Peucker_algorithm
///
/// These measurements are based on a 200km track from a Garmin Edge 1040,
/// which records 1 trackpoint every second. The original file is 11.5Mb, that
/// includes a lot of extension data such as heartrate which this program also
/// strips out. The percentages shown below are based solely on point counts.
///
/// The Audax UK DIY upload form allows a max file size of 1.25Mb.
///
/// Input Points    Metres  Output Points       Quality
/// 31358           1       4374 (13%, 563Kb)   Near-perfect map to the road
/// 31358           5       1484 (4.7%, 192Kb)  Very close map to the road, mainly stays within the road lines
/// 31358           10      978 (3.1%, 127Kb)   OK - good enough for submission
/// 31358           20      636 (2.0%, 83Kb)    Ok - within a few metres of the road
/// 31358           50      387 (1.2%, 51Kb)    Poor - cuts off a lot of corners
/// 31358           100     236 (0.8%, 31Kb)    Very poor - significant corner truncation
fn reduce_trackpoints_by_rdp(points: &mut Vec<TrackPoint>, epsilon: f32) {
    let line_string: LineString<f32> = points
        .iter()
        .map(|p| coord! { x: p.lon, y: p.lat })
        .collect();
    let indices_to_keep: HashSet<usize> = HashSet::from_iter(line_string.simplify_idx(&epsilon));

    let mut n = 0;
    points.retain(|_| {
        let keep = indices_to_keep.contains(&n);
        n += 1;
        keep
    });
}

fn format_utc_date(date: OffsetDateTime) -> String {
    let mut buf = Vec::with_capacity(32);
    write_utc_date(&mut buf, date);
    String::from_utf8(buf).unwrap()
}

fn to_local_date(date: OffsetDateTime) -> OffsetDateTime {
    let local_offset = UtcOffset::local_offset_at(date).unwrap();
    date.to_offset(local_offset)
}

fn format_utc_and_local_date(date: OffsetDateTime, sep: &str) -> String {
    let local_fmt = format_description::parse("[year]-[month]-[day] [hour repr:24]:[minute]:[second][end] (local time)").unwrap();
    let mut buf = Vec::with_capacity(64);
    write_utc_date(&mut buf, date);
    write!(buf, "{}", sep).unwrap();
    let d = to_local_date(date);
    d.format_into(&mut buf, &local_fmt).unwrap();
    String::from_utf8(buf).unwrap()
}

fn write_utc_date<W: Write>(w: &mut W, date: OffsetDateTime) {
    const DATE_FMT: Rfc3339 = time::format_description::well_known::Rfc3339;
    date.format_into(w, &DATE_FMT).unwrap();
}

/// The serde/quick-xml deserialization integration does a "good enough" job of parsing
/// the XML file. We also tag on the original filename as it's handy to track this
/// through the program for when we come to the point of writing output.
fn read_gpx_file(input_file: &Path) -> Gpx {
    let reader = Reader::from_file(input_file).expect("Could not create XML reader");
    let mut doc: Gpx = quick_xml::de::from_reader(reader.into_inner()).unwrap();
    doc.filename = input_file.to_owned();
    doc
}

fn write_output_file(output_file: &Path, gpx: &MergedGpx) {
    const HDR: &str = include_str!("header.txt");
    print!("Writing file {:?}", &output_file);

    let mut w = BufWriter::new(File::create(output_file).expect("Could not open output_file"));
    writeln!(w, "{}", HDR).unwrap();
    writeln!(w, "  <metadata>").unwrap();
    writeln!(w, "    <time>{}</time>", format_utc_date(gpx.metadata_time)).unwrap();
    writeln!(w, "  </metadata>").unwrap();
    writeln!(w, "  <trk>").unwrap();
    writeln!(w, "    <name>{}</name>", gpx.track_name).unwrap();
    writeln!(w, "    <type>{}</type>", gpx.track_type).unwrap();
    writeln!(w, "    <trkseg>").unwrap();
    for tp in &gpx.points {
        writeln!(w, "      <trkpt lat=\"{}\" lon=\"{}\">", tp.lat, tp.lon).unwrap();
        writeln!(w, "        <ele>{}</ele>", tp.ele).unwrap();
        writeln!(w, "        <time>{}</time>", format_utc_date(tp.time)).unwrap();
        writeln!(w, "        <speed>{}</speed>", tp.speed_kmh).unwrap(); // TODO: For testing
        writeln!(w, "      </trkpt>").unwrap();
    }
    writeln!(w, "    </trkseg>").unwrap();
    writeln!(w, "  </trk>").unwrap();
    writeln!(w, "</gpx>").unwrap();

    w.flush().unwrap();
    let metadata = std::fs::metadata(output_file).unwrap();
    println!(", {}Kb", metadata.len() / 1024);
}

/// Get a list of all files in the exe_dir that have the ".gpx" extension.
/// Be careful to exclude files that actually end in ".simplified.gpx" -
/// they are output files we already created! If we don't exclude them here,
/// we end up generating ".simplified.simplified.gpx", etc.
/// Remarks: the list of files is guaranteed to be sorted, this is
/// important for the joining algorithm (the first file is expected to
/// be the first part of the track, and so on).
fn get_list_of_input_files(exe_dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = read_dir(exe_dir) else {
        return files;
    };

    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        if meta.is_file() {
            let s = &entry.file_name();
            let p = Path::new(s);
            if let Some(ext) = p.extension() {
                if ext.to_ascii_lowercase() == "gpx" {
                    let s = s.to_string_lossy().to_ascii_lowercase();
                    if !s.ends_with(".simplified.gpx") {
                        println!("Found GPX input file {:?}", entry.path());
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files.sort_unstable();

    files
}

fn get_exe_dir() -> PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path
}
