#![cfg_attr(all(feature="serde_type"), feature(proc_macro))]

extern crate xml;
extern crate chrono;
#[macro_use]
extern crate iron;
#[macro_use]
extern crate router;
extern crate iron_sessionstorage;
extern crate urlencoded;
extern crate handlebars_iron as hbs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate maplit;

use std::cmp::Ordering;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::Read;

use iron::prelude::*;
use iron::typemap::Key;
use iron::middleware::*;
use iron::status;
use iron::modifiers::Redirect;
use iron_sessionstorage::traits::*;
use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;

use urlencoded::UrlEncodedBody;

use router::Router;

use hbs::{Template, HandlebarsEngine, DirectorySource, MemorySource};
use hbs::handlebars::{Handlebars, RenderContext, RenderError, Helper};
use hbs::handlebars::to_json;
#[cfg(feature = "watch")]
use hbs::Watchable;

use serde_json::value::{Value, Map};
use serde_json::Error;
use chrono::prelude::*;

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::mpsc::channel;
use std::thread;

mod ftp;
mod gpx;
mod quadtree;
mod geo;

use quadtree::QuadTree;
use geo::*;
use gpx::GPXPoint;

struct Login {
    username: String
}

impl iron_sessionstorage::Value for Login {
    fn get_key() -> &'static str { "logged_in_user" }
    fn into_raw(self) -> String { self.username }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(Login { username: value })
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    title: String,
    link: String,
    text: String,
    tags: Vec<String>,
    date: DateTime<UTC>,
    lat: f32,
    lng: f32,
}

impl Ord for Post {
    fn cmp(&self, other: &Post) -> Ordering {
        self.date.cmp(&other.date)
    }
}

impl PartialOrd for Post {
    fn partial_cmp(&self, other: &Post) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Post {
    fn eq(&self, other: &Post) -> bool {
        self.link == other.link
    }
}
impl Eq for Post {}

fn get_post(id: &str) -> Value {
    let mut file = File::open(format!("./resources/posts/{}.json", id));
    let mut buf_reader = BufReader::new(file.unwrap());
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();
    let p: Post = serde_json::from_str(contents.as_str()).unwrap();
    to_json(&p)
}

fn get_posts() -> Map<String, Value> {
    let mut data = Map::new();
    let paths = fs::read_dir("./resources/posts/").unwrap();
    let mut posts = Vec::new();
    for path in paths {
        let mut file = File::open(path.unwrap().path()).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        let mut p: Post = serde_json::from_str(contents.as_str()).unwrap();
        p.text = p.text.split_whitespace() // Turn the text into a snippet of less than 80 words
                       .map(|s| s.to_string())
                       .take(80)
                       .collect::<Vec<String>>()
                       .join(" ");
        posts.push(p);
    }
    posts.sort();
    posts.reverse();
    data.insert(String::from("posts"), to_json(&posts));
    data
}

fn check_login(username: &str, password: &str) -> bool{
    // laziest login ever, compile them right in
    let passwords : Vec<(String, String)> =
        serde_json::from_str(include_str!("../resources/passwords.json")).unwrap();
    let mut login = false;
    for (ref user, ref pass) in passwords {
        if username == *user && password == *pass {
            login = true;
            break
        }
    }
    login
}

fn login(req: &mut Request) -> IronResult<Response> {
    if try!(req.session().get::<Login>()).is_some() {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "home")))));
    }

    Ok(Response::with((
        status::Ok,
        "text/html".parse::<iron::mime::Mime>().unwrap(),
        format!("If you aren't me, why are you trying to login? Best of luck though...<br/> \n\
        <form method=post> \n\
        <input type=text name=username> \n\
        <input type=password name=password> \n\
        <input type=submit> \n\
        </form>")
    )))
}

fn login_post(req: &mut Request) -> IronResult<Response> {
    let username = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        iexpect!(formdata.get("username"))[0].to_owned()
    };

    let password = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        iexpect!(formdata.get("password"))[0].to_owned()
    };

    if check_login(username.as_str(), password.as_str()) {
        try!(req.session().set(Login { username: username }));
        Ok(Response::with((status::Found, Redirect(url_for!(req, "home")))))
    } else {
        Ok(Response::with((
                    status::Unauthorized,
                    "text/html".parse::<iron::mime::Mime>().unwrap(),
                    "bad password.")))
    }
}

fn logout(req: &mut Request) -> IronResult<Response> {
    try!(req.session().clear());
    Ok(Response::with((status::Found, Redirect(url_for!(req, "home")))))
}

fn editor(req: &mut Request) -> IronResult<Response> {
    if try!(req.session().get::<Login>()).is_some() {
        let data = Map::new();
        let mut resp = Response::new();
        resp.set_mut(Template::new("editor", data)).set_mut(status::Ok);
        Ok(resp)
    } else {
        Ok( Response::with((status::Unauthorized,
                            "text/html".parse::<iron::mime::Mime>().unwrap(),
                            "<a href=/login>Log in</a>")))
    }
}

fn home(req: &mut Request) -> IronResult<Response> {
    let mut data = get_posts();
    let mut resp = Response::new();
    if try!(req.session().get::<Login>()).is_some() {
        data.insert(String::from("loggedin"), to_json(&true));
    }
    resp.set_mut(Template::new("home", data)).set_mut(status::Ok);
    Ok(resp)
}

fn the_map(req: &mut Request) -> IronResult<Response> {
    let mut data = get_posts();
    let mut resp = Response::new();
    if try!(req.session().get::<Login>()).is_some() {
        data.insert(String::from("loggedin"), to_json(&true));
    }
    resp.set_mut(Template::new("the_map", data)).set_mut(status::Ok);
    Ok(resp)
}

fn points_handler(req: &mut Request) -> IronResult<Response> {
    let mut data = get_posts();
    let mut resp = Response::new();
    let mut lat = 0f64;
    let mut lng = 0f64;
    let mut rad = 0f64;
    {
        let router = req.extensions.get::<Router>().unwrap();
        lat = router.find("lat").unwrap_or("0").parse().unwrap();
        lng = router.find("lng").unwrap_or("0").parse().unwrap();
        rad = router.find("radius").unwrap_or("0").parse().unwrap();
    }

    let tree_lock = req.get_tree();
    let tree_read = tree_lock.read().unwrap();

    //TODO better geo system
    let radians = rad / 110000.0;
    let mut points = tree_read.get(lng, lat, radians);
    points.sort();
    let point_resp = geo::jsonify(points);
    Ok(Response::with((
        status::Ok,
        "text/json".parse::<iron::mime::Mime>().unwrap(),
        point_resp)))
}

fn post_handler(req: &mut Request) -> IronResult<Response> {
    let query = req.extensions.get::<Router>().unwrap().find("post_id").unwrap_or("/");
    let mut data = get_post(query);
    let mut resp = Response::new();
    resp.set_mut(Template::new("post", data)).set_mut(status::Ok);
    Ok(resp)
}

fn post_create(req: &mut Request) -> IronResult<Response> {
    let query = req.extensions.get::<Router>().unwrap().find("post_id").unwrap_or("/");
    let mut s = String::from("");
    let x = req.body.read_to_string(&mut s);
    let post: Post = serde_json::from_str(s.as_str()).unwrap();
    let mut f = File::create(format!("./resources/posts/{}.json", post.link)).unwrap();
    f.write(serde_json::to_string(&post).unwrap().as_bytes());
    Ok(Response::with((
        status::Ok,
        "text/json".parse::<iron::mime::Mime>().unwrap(),
        "{\"status\": \"ok\"}")))
}

#[derive(Clone)]
struct TreeWare {
    tree: Arc<RwLock<QuadTree<GPXPoint>>>
}
impl Key for TreeWare {
    type Value = Arc<RwLock<QuadTree<GPXPoint>>>;
}

impl BeforeMiddleware for TreeWare {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<TreeWare>(self.tree.clone());
        Ok(())
    }
}

pub trait TreeWareExt {
    fn get_tree(&mut self) -> &mut Arc<RwLock<QuadTree<GPXPoint>>>;
}

impl<'a, 'b> TreeWareExt for Request<'a, 'b> {
    fn get_tree(&mut self) -> &mut Arc<RwLock<QuadTree<GPXPoint>>> {
        self.extensions.get_mut::<TreeWare>().unwrap()
    }
}

fn main() {

    let tree_lock = Arc::new(RwLock::new(quadtree::QuadTree::root()));

    let (tx, rx) = channel();
    thread::spawn(move || {
        ftp::start_ftpserver(tx)
    });
    let tree2 = tree_lock.clone();
    thread::spawn(move || {
        loop {
            let gps_file = rx.recv().unwrap();
            for point in gpx::parse(gps_file) {
                let mut map_tree = tree2.write().unwrap();
                map_tree.insert(point);
            }
            // loop was going crazy, calm it down
            thread::sleep_ms(10000);
        }
    });
    let tree2 = tree_lock.clone();
    for entry in fs::read_dir("./gpx/").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        for point in gpx::parse(path.to_str().unwrap().to_string()) {
            let mut map_tree = tree2.write().unwrap();
            map_tree.insert(point);
        }
    }

    let x = tree2.read().unwrap();

    let router = router!{
        home: get "/" => home,
        post: get "/post/:post_id" => post_handler,
        map: get "/the-map" => the_map,
        points: get "/points/:lat/:lng/:radius" => points_handler,
        editor: get "/editor" => editor,
        save_post: post "/post" => post_create,
        login: get "/login" => login,
        login_post: post "/login" => login_post,
        logout: get "/logout" => logout,
    };

    let my_secret = include_bytes!("../resources/passwords.json").to_vec();
    let mut ch = Chain::new(router);
    ch.link_before(TreeWare {tree: tree_lock.clone() });
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./resources/templates/", ".hbs")));

    #[cfg(feature = "watch")]
    {
        let hbse_ref = Arc::new(hbse);
        hbse_ref.watch("./resources/templates/");
        ch.link_after(hbse_ref);
    }

    #[cfg(not(feature = "watch"))]
    {
        if let Err(r) = hbse.reload() {
            panic!("{}", r);
        }
        ch.link_after(hbse);
    }

    let _res = Iron::new(ch).http("localhost:8080");
    println!("Listening on 8080.");
}
