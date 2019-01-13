#![feature(proc_macro_hygiene, decl_macro)]

use rocket::fairing::AdHoc;
use rocket::http::{hyper::header, Method, Status};
use rocket::{self, get, routes, State};
use serde_json::{self, json};
use std::collections::HashMap;
use std::path::PathBuf;

enum VideoEntry {
    Folder(VideoFolder),
    File(VideoFile),
}

impl VideoEntry {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            VideoEntry::Folder(c) => json!({
                "folderType": c.folder_type,
                "label":      c.label,
                "numEntries": c.entries.len(),
            }),
            VideoEntry::File(e) => json!({
                "fileType": e.file_type,
                "label":    e.label,
                "url":      e.url,
            }),
        }
    }
}

struct VideoFile {
    file_type: &'static str,
    label:     String,
    url:       String
}

struct VideoFolder {
    folder_type: &'static str,
    label:       String,
    entries:     HashMap<String, VideoEntry>,
}

struct Store {
    video_root: HashMap<String, VideoEntry>,
}

impl Store {
    pub fn new() -> Store {
        let mut video_root = HashMap::new();

        let mut ynn_entries = HashMap::new();
        ynn_entries.insert("episode-01".to_owned(), VideoEntry::File(VideoFile {
            file_type: "video",
            label:     "Episode 1".to_owned(),
            url:       "https://dark.red/anime/yakusoku-no-neverland/episode-01.mkv".to_owned(),
        }));
        ynn_entries.insert("episode-02".to_owned(), VideoEntry::File(VideoFile {
            file_type: "video",
            label:     "Episode 2".to_owned(),
            url:       "https://dark.red/anime/yakusoku-no-neverland/episode-02.mkv".to_owned(),
        }));

        let mut anime_entries = HashMap::new();
        anime_entries.insert("yakusoku-no-neverland".to_owned(), VideoEntry::Folder(VideoFolder {
            folder_type: "series",
            label:       "Yakusoku no Neverland".to_owned(),
            entries:     ynn_entries,
        }));

        video_root.insert("anime".to_owned(), VideoEntry::Folder(VideoFolder {
            folder_type: "media-type",
            label:       "Anime".to_owned(),
            entries:     anime_entries,
        }));

        let mut twitch_entries = HashMap::new();
        twitch_entries.insert("gamesdonequick".to_owned(), VideoEntry::File(VideoFile {
            file_type: "livestream",
            label:     "GamesDoneQuick".to_owned(),
            url:       "https://twitch.tv/gamesdonequick".to_owned(),
        }));
        twitch_entries.insert("thatguytagg".to_owned(), VideoEntry::File(VideoFile {
            file_type: "livestream",
            label:     "ThatGuyTagg".to_owned(),
            url:       "https://twitch.tv/thatguytagg".to_owned(),
        }));

        video_root.insert("twitch".to_owned(), VideoEntry::Folder(VideoFolder {
            folder_type: "media-type",
            label:       "Twitch".to_owned(),
            entries:     twitch_entries,
        }));

        Store {
            video_root,
        }
    }
}

#[get("/<video..>")]
fn video_browse(video: PathBuf, store: State<Store>) -> Option<String> {
    let mut path = video.into_iter();
    if path.next().and_then(|s| s.to_str()) != Some("video") {
        return None;
    }

    let mut crumbs = vec![];
    let mut folder = &store.video_root;
    for seg in path {
        let seg = seg.to_string_lossy().into_owned();
        match folder.get(&seg) {
            Some(VideoEntry::Folder(c)) => {
                folder = &c.entries;
                crumbs.push(c.label.clone());
            },
            Some(VideoEntry::File(_)) => return None,
            None                      => return None,
        }
    }

    Some(
        json!({
            "crumbs": crumbs,
            "entries": folder.iter().map(|(k, v)| {
                (k, v.to_json())
            }).collect::<HashMap<_, _>>(),
        })
        .to_string()
    )
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
               video_browse,
        ])
        .manage(Store::new())
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
