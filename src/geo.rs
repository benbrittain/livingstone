use serde_json;
use gpx::GPXPoint;
use quadtree::Geospatial;
use std::f64;

static A: f64 = 6378137.0;
static MAXEXTENT: f64 = 20037508.342789244;
static D2R: f64 = f64::consts::PI / 180.0;
static R2D: f64 = 180.0 / f64::consts::PI;


pub fn lat_to_y(lat: f64) -> f64 {
    ((f64::consts::PI * 0.5) - 2.0 * ((-lat / A).exp()).atan()) * R2D
}

pub fn lng_to_x(lng: f64) -> f64 {
    lng * R2D / A
}

pub fn x_to_lng(x: f64) -> f64 {
    A * x * D2R.max(-MAXEXTENT).min(MAXEXTENT) as f64
}

pub fn y_to_lat(y: f64) -> f64 {
    A * (((f64::consts::PI * 0.25f64) + (0.5f64 * y * D2R)).tan()).ln()
}

pub fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi_1 = lat1.to_radians();
    let phi_2 = lat2.to_radians();
    let delta_phi = (lat2-lat1).to_radians();
    let delta_lambda = (lon2-lon1).to_radians();

    let a = (delta_phi/2.0).sin().powi(2)* phi_1.cos() * phi_2.cos() * (delta_lambda/2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0-a).sqrt());
    A * c
}

pub fn simplify<T>(points: Vec<T>) -> Vec<T> {
    //TODO: implement Ramer–Douglas–Peucker
    points
}

pub fn jsonify<T>(points: Vec<T>) -> String
    where T: Geospatial {

    let mut simple_points: Vec<(f64, f64)> = Vec::new();
    for point in points {
        simple_points.push((point.x(), point.y()))
    }
    simple_points = simplify(simple_points);

	format!("
	{{
		\"type\": \"Feature\",
		\"geometry\": {{
			\"type\": \"LineString\",
			\"coordinates\": {}
		}},
		\"properties\": {{}}
	}}", serde_json::to_string(&simple_points).unwrap())
}
