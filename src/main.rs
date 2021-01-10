use std::{format, sync::Arc};

use bytes::BufMut;
use chrono::prelude::*;
use futures::TryStreamExt;
use log::{info};
use warp::{http, multipart::Part};
use warp::{multipart::FormData, Buf};
use warp::{Filter, Rejection, Reply};
mod config;

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

async fn upload(
    form: FormData,
    db: Arc<sled::Db>,
    url: warp::path::FullPath,
) -> Result<impl Reply, Rejection> {
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
                    eprintln!("reading file error: {}", e);
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
                url: String::from(url.as_str()),
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

async fn view_data(key: String, db: Arc<sled::Db>) -> Result<impl Reply, Rejection> {
    if let Ok(Some(data)) = db.get(key.as_str()) {
        return Ok(warp::reply::with_status(String::from_utf8_lossy(&data).to_string(), http::StatusCode::FOUND));
    } else {
        return Ok(warp::reply::with_status(String::from("not found"), http::StatusCode::NOT_FOUND));
    }
}

#[tokio::main]
async fn main() {
    let db: Arc<sled::Db> = Arc::new(sled::open("db").unwrap());
    let db_filter = warp::any().map(move || db.clone());
    let upload_route = warp::path::end()
        .and(warp::post())
        .and(warp::multipart::form().max_length(5_000_000))
        .and(db_filter.clone())
        .and(warp::path::full())
        .and_then(upload);
    let view_route = warp::path!(String)
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(view_data);
    let route = upload_route.or(view_route);
    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
