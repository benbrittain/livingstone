use std::thread;
use chrono::*;
use xml::reader::{EventReader, XmlEvent};
use std::fs::File;
use std::io::BufReader;

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
//				println!("{} START {}", indent(depth), name);
				curname = name.local_name;
				if curname.contains("trkpt") {
//					println!("trkpt found");
					let mut pt = GPXPoint { lat: 0.0f64, lon: 0.0f64, elev: None, time: None };
					for attr in attributes {
//						println!("{} ATTR: {} = {}", indent(depth + 1), attr.name, attr.value);
						match attr.name.local_name.as_ref() {
							"lat" => {
								pt.lat = attr.value.parse().unwrap();
//								println!("lat: {}", pt.lat);
							}
							"lon" => {
								pt.lon = attr.value.parse().unwrap();
//								println!("lon: {}", pt.lon);
							}
							_ => {
//								println!("warn: unknown attr {}", attr.name);
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
//							println!("pushing trkpt: {}, {}, {:?}, {:?}", pt.lat, pt.lon, pt.elev, pt.time);
							points.push(pt);
						},
						None => {}
					}
					elem = None;
				}
//				println!("{}-{}", indent(depth), name);
			}
			Ok(XmlEvent::CData(string)) => {
//				println!("{} CDATA: {}", indent(depth), string);
			}
			Ok(XmlEvent::Characters(string)) => {
//				println!("{} {} CHARS: {}", indent(depth), curname, string);
				match elem {
					Some(ref mut pt) => {
						match curname.as_ref() {
							"ele" => {
//								println!("ele: {}", string);
								pt.elev = Some(string.parse().unwrap());
							},
							"time" => {
//								println!("time: {}", string);
								let result = DateTime::parse_from_rfc3339(&string);
								match result {
									Ok(time) => {
//										println!("time parsed\n");
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
