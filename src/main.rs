#![allow(proc_macro_derive_resolution_fallback)] // oh my god DIESEL
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate diesel;

use diesel::{prelude::*, SqliteConnection};
use itertools::Itertools;
use rocket::fairing::AdHoc;
use rocket::http::{hyper::header, Method, Status};
use rocket::{self, get, routes};
use rocket_contrib::{database, json::Json};
use serde::Serializer;
use serde_derive::Serialize;
use serde_json::value::RawValue;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};

mod schema;

type StdResult<T> = Result<T, Box<dyn StdError>>;

#[database("store")]
struct DbConn(SqliteConnection);

#[derive(Serialize)]
struct BrowseVideoResponse {
    entry: Option<VideoEntry>,
    crumbs: Vec<String>,
    children: Vec<VideoEntry>,
}

#[derive(Serialize, Queryable)]
struct VideoEntry {
    #[serde(skip)]
    id:             i32,
    #[serde(skip)]
    parent:         i32,
    #[serde(rename = "entryType")]
    entry_type:     String,
    key:            String,
    label:          String,
    url:            Option<String>,
    #[serde(serialize_with = "json_raw")]
    extra:          Option<String>,
    #[serde(skip)]
    provider_extra: Option<String>,
}

fn json_raw<S: Serializer>(opt: &Option<String>, ser: S) -> Result<S::Ok, S::Error> {
    ::serde::Serialize::serialize(&opt.as_ref().map(|s| RawValue::from_string(s.clone()).expect("invalid extra json")), ser)
}

impl VideoEntry {
    pub fn by_parent_key(conn: &SqliteConnection, parent: i32, key: &str) -> StdResult<Option<VideoEntry>> {
        use crate::schema::entries::dsl;

        Ok(dsl::entries
            .filter(dsl::parent.eq(parent))
            .filter(dsl::key.eq(key))
            .first(conn)
            .optional()?)
    }

    pub fn by_path(conn: &SqliteConnection, path: &Path) -> StdResult<Option<(VideoEntry, Vec<String>)>> {
        let mut parent_id = 0;
        let mut crumbs = vec![];
        let mut entry = None;

        for seg in path.iter() {
            entry = VideoEntry::by_parent_key(conn, parent_id, &seg.to_string_lossy())?;

            match &entry {
                Some(e) => {
                    crumbs.push(e.label.clone());
                    parent_id = e.id;
                },
                None => return Ok(None),
            }
        }

        Ok(Some((entry.unwrap(), crumbs)))
    }

    pub fn children_by_parent(conn: &SqliteConnection, parent: i32) -> StdResult<Vec<VideoEntry>> {
        use crate::schema::entries::dsl;

        Ok(dsl::entries
           .filter(dsl::parent.eq(parent))
           .load(conn)?)
    }

    pub fn children(&self, conn: &SqliteConnection) -> StdResult<Vec<VideoEntry>> {
        VideoEntry::children_by_parent(conn, self.id)
    }
}

#[get("/video")]
fn video_root(db: DbConn) -> StdResult<Json<BrowseVideoResponse>> {
    let children = VideoEntry::children_by_parent(&db, 0)?;

    let res = BrowseVideoResponse {
        entry: None,
        crumbs: vec![],
        children,
    };

    Ok(Json(res))
}

#[get("/video/<path..>")]
fn video_browse(path: PathBuf, db: DbConn) -> StdResult<Option<Json<BrowseVideoResponse>>> {
    let entry = VideoEntry::by_path(&db, &path)?;
    if entry.is_none() {
        return Ok(None);
    }

    let (entry, crumbs) = entry.unwrap();
    let children        = entry.children(&db)?;
    let entry           = Some(entry);

    let res = BrowseVideoResponse {
        entry, crumbs, children,
    };

    Ok(Some(Json(res)))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
               video_root, video_browse,
        ])
        .attach(DbConn::fairing())
        .attach(AdHoc::on_response("Set headers", |req, res| {
            res.set_header(header::AccessControlAllowOrigin::Value("http://localhost:4200".to_owned()));

            if req.method() == Method::Options && res.status() == Status::NotFound {
                res.set_status(Status::Ok);
                res.set_raw_header("Access-Control-Allow-Headers", "Content-Type");
                res.set_raw_header("Access-Control-Allow-Methods", "HEAD,GET,PUT,POST,DELETE,OPTIONS");
                res.take_body();
            }
        }))
        .launch();
}
