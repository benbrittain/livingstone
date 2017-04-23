#![cfg_attr(all(feature="serde_type"), feature(proc_macro))]

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

use iron::prelude::*;
use iron::status;
use iron::modifiers::Redirect;

use std::io::Read;
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

use std::sync::Arc;
use serde_json::value::{Value, Map};
use serde_json::Error;

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
    date: String,
    lat: f32,
    lng: f32,
}

fn get_post(id: &str) -> Value {
    to_json(&Post {
        title: String::from(id),
        link: String::from("africa"),
        text: String::from("africa\n
        eunsaotehu"),
        date: String::from("10/3/2017"),
        tags: vec![String::from("Nigeria"), String::from("Africa Trip")],
        lat: -23.9,
        lng: 28.7,
    })
}


fn get_posts() -> Map<String, Value> {
    let mut data = Map::new();

    let posts= vec![Post {
        title: String::from("Toto - Africa"),
        text: String::from("africa\n
        eunsaotehu"),
        date: String::from("10/3/2017"),
        tags: vec![String::from("Nigeria"), String::from("Africa Trip")],
        link: String::from("africa"),
        lat: -23.9,
        lng: 28.7,
    },
    Post{
        title: String::from("Let's hear it for Africa"),
        text: String::from("africa\n
        eunsaotehu"),
        date: String::from("10/3/2017"),
        tags: vec![String::from("Nigeria"), String::from("Africa Trip")],
        link: String::from("blah"),
        lat: -21.9,
        lng: 26.7,
    }];
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
        // Already logged in
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

fn post_handler(req: &mut Request) -> IronResult<Response> {
    let query = req.extensions.get::<Router>().unwrap().find("post_id").unwrap_or("/");
    println!("{}", query);

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
    Ok(Response::with((
        status::Ok,
        "text/json".parse::<iron::mime::Mime>().unwrap(),
        "{\"status\": \"ok\"}")))
}

fn main() {
    let router = router!{
        home: get "/" => home,
        post: get "/post/:post_id" => post_handler,
        map: get "/the-map" => home,
        editor: get "/editor" => editor,
        save_post: post "/post" => post_create,
        login: get "/login" => login,
        login_post: post "/login" => login_post,
        logout: get "/logout" => logout,
    };

    let my_secret = include_bytes!("../resources/passwords.json").to_vec();
    let mut ch = Chain::new(router);
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./resources/templates/", ".hbs")));

    // TODO remove these, replace with reload
    let hbse_ref = Arc::new(hbse);
    hbse_ref.watch("./resources/templates/");

    ch.link_after(hbse_ref);

    //if let Err(r) = hbse.reload() {
    //    panic!("{}", r);
    //}
    //ch.link_after(hbse);

    let _res = Iron::new(ch).http("localhost:8080");
    println!("Listening on 8080.");
}
