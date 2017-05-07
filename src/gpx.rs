use std::thread;
use chrono::*;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::BufReader;
use std::cmp::Ordering;

fn indent(size: usize) -> String {
	const INDENT: &'static str = "    ";
	(0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size*INDENT.len()), |r, s| r + s)
}

#[derive(Debug, Clone, Copy)]
pub struct GPXPoint {
	pub lat: f64,
	pub lon: f64,
	pub elev: Option<f64>,
	pub time: Option<DateTime<FixedOffset>>
}

impl Eq for GPXPoint {}

impl PartialEq for GPXPoint {
    fn eq(&self, other: &GPXPoint) -> bool {
        self.time == other.time
    }
}

impl Ord for GPXPoint {
    fn cmp(&self, other: &GPXPoint) -> Ordering {
        self.time.cmp(&other.time)
    }
}


impl PartialOrd for GPXPoint {
    fn partial_cmp(&self, other: &GPXPoint) -> Option<Ordering> {
        Some(self.time.cmp(&other.time))
    }
}


pub fn parse(file_path: String) -> Vec<GPXPoint> {
	let file = File::open(file_path).unwrap();
	let file = BufReader::new(file);

	let parser = EventReader::new(file);
	let mut depth = 0;
    let mut points: Vec<GPXPoint> = Vec::new();
    let mut elem: Option<GPXPoint> = None;
    let mut curname: String = String::from("");
	for e in parser {
		match e {
			Ok(XmlEvent::StartElement { name, attributes, .. }) => {
				curname = name.local_name;
				if curname.contains("trkpt") {
					let mut pt = GPXPoint { lat: 0.0f64, lon: 0.0f64, elev: None, time: None };
					for attr in attributes {
						match attr.name.local_name.as_ref() {
							"lat" => {
								pt.lat = attr.value.parse().unwrap();
							}
							"lon" => {
								pt.lon = attr.value.parse().unwrap();
							}
							_ => {
								println!("warn: unknown attr {}", attr.name);
							}
						}
					}
					elem = Some(pt);
				}
				depth += 1;
			}
			Ok(XmlEvent::EndElement { name }) => {
				depth -= 1;
				if name.local_name.contains("trkpt") {
					match elem {
						Some(pt) => {
							points.push(pt);
						},
						None => {}
					}
					elem = None;
				}
			}
			Ok(XmlEvent::CData(string)) => {
			}
			Ok(XmlEvent::Characters(string)) => {
				match elem {
					Some(ref mut pt) => {
						match curname.as_ref() {
							"ele" => {
								pt.elev = Some(string.parse().unwrap());
							},
							"time" => {
								let result = DateTime::parse_from_rfc3339(&string);
								match result {
									Ok(time) => {
										pt.time = Some(time);
									}
									Err(e) => {
										panic!("error parsing time: {}", e);
									}
								}
							},
							_ => {}
						}
					}
					None => {}
				}
			}
			Err(e) => {
				println!("Error: {}", e);
				break;
			}
			_ => {}
		}
	}
	points
}
