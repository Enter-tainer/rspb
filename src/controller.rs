use std::sync::Arc;

use bytes::BufMut;
use chrono::prelude::*;
use futures::TryStreamExt;
use log::info;
use warp::{http, multipart::Part};
use warp::{multipart::FormData, Buf};
use warp::{Rejection, Reply};

use crate::highlighter::highlight_lines;

enum UploadStatus {
    Created,
    Existed,
}

impl ToString for UploadStatus {
    fn to_string(&self) -> String {
        match self {
            UploadStatus::Created => String::from("created"),
            UploadStatus::Existed => String::from("existed"),
        }
    }
}

struct UploadResponse {
    date: String,
    digest: String,
    short: String,
    size: usize,
    status: UploadStatus,
    url: String,
}

impl ToString for UploadResponse {
    fn to_string(&self) -> String {
        return format!(
            "date: {}
digest: {}
short: {}
size: {}
url: {}
status: {}
",
            self.date,
            self.digest,
            self.short,
            self.size,
            self.url,
            self.status.to_string()
        );
    }
}

pub async fn upload(form: FormData, db: sled::Db, url: String) -> Result<impl Reply, Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
    })?;
    for p in parts {
        if p.name() == "c" || p.name() == "content" {
            let value = p
                .stream()
                .try_fold(Vec::new(), |mut vec, data| {
                    vec.put(data.bytes());
                    async move { Ok(vec) }
                })
                .await
                .map_err(|e| {
                    log::error!("reading file error: {}", e);
                    warp::reject::reject()
                })?;
            let hash = blake3::hash(&value);
            // let hash_bytes = hash.as_bytes();
            let short = &hash.to_hex()[0..4];
            let date: DateTime<Local> = Local::now();
            let existed = db.get(short).unwrap().is_some();
            if !existed {
                db.insert(short, value.clone()).unwrap();
            }

            let response = UploadResponse {
                date: date.to_string(),
                digest: hash.to_hex().to_string(),
                short: String::from(short),
                size: value.len(),
                status: if existed {
                    UploadStatus::Existed
                } else {
                    UploadStatus::Created
                },
                url: format!("{}/{}", url, short),
            };
            info!(
                "{} {} of length {}",
                response.status.to_string(),
                response.short,
                response.size
            );
            if existed {
                return Ok(warp::reply::with_status(
                    response.to_string(),
                    http::StatusCode::FOUND,
                ));
            } else {
                return Ok(warp::reply::with_status(
                    response.to_string(),
                    http::StatusCode::CREATED,
                ));
            }
        }
    }
    Ok(warp::reply::with_status(
        String::from("invalid"),
        http::StatusCode::BAD_REQUEST,
    ))
}

pub async fn view_data(key: String, db: sled::Db) -> Result<warp::reply::Response, Rejection> {
    let mut database_key: String = key.clone().to_lowercase();
    let mut ext: String = String::from("txt");
    let mut highlighting = false;
    if key.contains('.') {
        let res: Vec<&str> = key.split('.').collect();
        database_key = String::from(res[0]);
        ext = String::from(res[res.len() - 1]);
        highlighting = true;
    }
    if let Ok(Some(data)) = db.get(database_key.as_str()) {
        info!("get {} success", key);
        if highlighting {
            let html = highlight_lines(&String::from_utf8_lossy(&data).to_string(), &ext);
            return Ok(warp::reply::html(html).into_response());
        }
        return Ok(warp::reply::with_status(
            String::from_utf8_lossy(&data).to_string(),
            http::StatusCode::OK,
        )
        .into_response());
    } else {
        info!("get {} failed", key);
        return Ok(warp::reply::with_status(
            String::from("not found"),
            http::StatusCode::NOT_FOUND,
        )
        .into_response());
    }
}

// pub async fn shorten_url(
//     form: FormData,
//     db: sled::Db,
//     url: String,
// ) -> Result<impl Reply, Rejection> {
// }
