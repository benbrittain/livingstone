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

#[derive(Serialize)]
struct LineString {
    #[serde(rename="type")]
    _type: String,
    coordinates: Vec<(f64, f64)>
}

#[derive(Serialize)]
struct Properties {
    date: String,
    color: String,
}

#[derive(Serialize)]
struct Feature {
    #[serde(rename="type")]
    _type: String,
    geometry: LineString, //TODO replace with enum of geojson types
    properties: Properties,
}

#[derive(Serialize)]
struct FeatureCollection {
    #[serde(rename="type")]
    _type: String,
    features: Vec<Feature>,
}

pub fn jsonify<T>(points: Vec<T>) -> String
    where T: Geospatial + Ord {
    // points are already sorted by date

    let colors = [
        "#5E412F",
        "#FCEBB6",
        "#78C0A8",
        "#F07818",
        "#F0A830"];

    let mut simple_points: Vec<(f64, f64)> = Vec::new();
    let mut different_days: Vec<Feature> = Vec::new();

    let mut last_day = None;
    let mut last_day_points: Vec<(f64, f64)> = Vec::new();
    let mut idx = 0;

    for point in points.iter() {
        if last_day.is_none() {
            last_day = Some(point.date().date())
        }
        if last_day != Some(point.date().date()) {
            different_days.push(Feature {
                _type: String::from("Feature"),
                geometry: LineString {
                    _type: String::from("LineString"),
                    coordinates: last_day_points.clone()
                },
                properties: Properties{
                    date: point.date().date().to_string(),
                    color: String::from(colors[idx%5]),
                },
            });
            idx +=1;
            last_day = Some(point.date().date());
            last_day_points = Vec::new();
        }
        last_day_points.push((point.x(), point.y()))
    }
    different_days.push(Feature {
        _type: String::from("Feature"),
        geometry: LineString {
            _type: String::from("LineString"),
            coordinates: last_day_points.clone(),
        },
        properties: Properties{
            date: last_day.unwrap().to_string(),
            color: String::from(colors[idx%5]),
        },
    });
//    simple_points = simplify(simple_points);

    let fc = FeatureCollection {
        _type: String::from("FeatureCollection"),
        features: different_days,
    };

    serde_json::to_string(&fc).unwrap()
}
