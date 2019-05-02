#![feature(proc_macro_hygiene, decl_macro)]

use itertools::Itertools;
use rocket::fairing::AdHoc;
use rocket::http::{hyper::header, Method, Status};
use rocket::{self, get, routes};
use rocket_contrib::json::Json;
use serde::Serializer;
use serde_derive::Serialize;
use serde_json::value::RawValue;
use std::error::Error as StdError;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::fs;
use std::path::{Component, Path, PathBuf};

type StdResult<T> = Result<T, Box<dyn StdError>>;

const VIDEO_ROOT: &'static str = "/home/dark/tobio";

#[derive(Serialize)]
struct BrowseResponse {
    crumbs:   Vec<Crumb>,
    children: Vec<MediaEntry>,
}

#[derive(Serialize)]
struct Crumb {
    path:  String,
    label: String,
}

impl Crumb {
    fn from_path<P: AsRef<Path>>(path: P) -> Vec<Crumb> {
        let mut crumbs = path.as_ref()
            .ancestors()
            .map(|p| {
                let mut path = PathBuf::from("/");
                path.push(p);

                dbg!(&path);

                Crumb {
                    path:  path.to_string_lossy().to_string(),
                    label: p.file_name().map(|s| s.to_str().expect("utf8 filename")).unwrap_or("/").to_string(),
                }
            })
            .collect::<Vec<_>>();

        crumbs.reverse();
        crumbs
    }
}

#[derive(Serialize)]
struct MediaEntry {
    folder:   bool,
    path:     String,
    filename: String,
    // #[serde(serialize_with = "json_raw")]
    // extra:          Option<String>,
    // #[serde(skip)]
    // provider_extra: Option<String>,
}

impl MediaEntry {
    fn from(entry: DirEntry) -> StdResult<MediaEntry> {
        let path = entry.path().canonicalize()?
            .strip_prefix(VIDEO_ROOT).unwrap()
            .to_str()
            .expect("utf8 filename")
            .to_string();

        let filename = entry.file_name()
            .into_string()
            .expect("utf8 filename");

        let folder = entry.file_type()?.is_dir();

        Ok(MediaEntry {
            folder,
            path,
            filename,
        })
    }
}

// fn json_raw<S: Serializer>(opt: &Option<String>, ser: S) -> Result<S::Ok, S::Error> {
//     ::serde::Serialize::serialize(&opt.as_ref().map(|s| RawValue::from_string(s.clone()).expect("invalid extra json")), ser)
// }

impl MediaEntry {
    fn list_dir<P: AsRef<Path>>(path: P) -> StdResult<Vec<MediaEntry>> {
        let path = path.as_ref().strip_prefix("/").unwrap_or(path.as_ref());
        let path = PathBuf::from(VIDEO_ROOT).join(path);

        path.read_dir()?
            .filter_map(|e| e.ok())
            .map(|entry| MediaEntry::from(entry))
            .collect::<Result<Vec<_>, _>>()
    }
}

#[get("/browse")]
fn browse_root() -> StdResult<Json<BrowseResponse>> {
    let crumbs   = Crumb::from_path("/");
    let children = MediaEntry::list_dir("/")?;

    let res = BrowseResponse {
        crumbs,
        children,
    };

    Ok(Json(res))
}

#[get("/browse/<path..>")]
fn browse_path(path: PathBuf) -> StdResult<Json<BrowseResponse>> {
    let crumbs   = Crumb::from_path(&path);
    let children = MediaEntry::list_dir(&path)?;

    let res = BrowseResponse {
        crumbs,
        children,
    };

    Ok(Json(res))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
               browse_root, browse_path,
        ])
        .attach(AdHoc::on_response("Set headers", |req, res| {
            res.set_header(header::AccessControlAllowOrigin::Value("http://172.24.0.3:4200".to_owned()));

            if req.method() == Method::Options && res.status() == Status::NotFound {
                res.set_status(Status::Ok);
                res.set_raw_header("Access-Control-Allow-Headers", "Content-Type");
                res.set_raw_header("Access-Control-Allow-Methods", "HEAD,GET,PUT,POST,DELETE,OPTIONS");
                res.take_body();
            }
        }))
        .launch();
}
