use std::{collections::HashMap, unreachable};

use bytes::BufMut;
use chrono::{prelude::*, Duration};
use futures::TryStreamExt;
use log::info;

use model::{add_record, delete_record, DataBaseItem};
use warp::{http, hyper::Uri, multipart::Part, path::FullPath};
use warp::{multipart::FormData, Buf};
use warp::{Rejection, Reply};

use crate::{
    highlighter::highlight_lines,
    model::{self, DataType},
};

enum UploadStatus {
    Failed,
    Created,
    Existed,
}

impl ToString for UploadStatus {
    fn to_string(&self) -> String {
        match self {
            UploadStatus::Created => String::from("created"),
            UploadStatus::Existed => String::from("existed"),
            UploadStatus::Failed => String::from("failed"),
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
    uuid: String,
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
uuid: {}
",
            self.date,
            self.digest,
            self.short,
            self.size,
            self.url,
            self.status.to_string(),
            self.uuid,
        );
    }
}

async fn read_multipart_form(parts: Vec<Part>) -> HashMap<String, Vec<u8>> {
    let mut res = std::collections::HashMap::new();
    for p in parts {
        let name = String::from(p.name());
        let value = p
            .stream()
            .try_fold(Vec::new(), |mut vec, data| {
                vec.put(data.bytes());
                async move { Ok(vec) }
            })
            .await
            .unwrap_or(vec![]);
        res.insert(name, value);
    }
    res
}

async fn process_upload(
    path: FullPath,
    form: FormData,
    db: model::DataTrees,
    url: String,
    custom_url: Option<String>,
) -> Result<warp::reply::Response, Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
    })?;
    let data = read_multipart_form(parts).await;
    let content = data.get("c").or(data.get("content"));
    let destroy = data.get("sunset");
    let now: DateTime<Utc> = Utc::now();

    if content.is_none() {
        return Ok(
            warp::reply::with_status(String::from("error"), http::StatusCode::BAD_REQUEST)
                .into_response(),
        );
    }
    let data = DataType::from_bytes(content.unwrap().clone(), Some(path.as_str() == "/u"));
    if data.is_none() {
        return Ok(
            warp::reply::with_status(String::from("error"), http::StatusCode::BAD_REQUEST)
                .into_response(),
        );
    }
    let data = data.unwrap();
    let mut item: DataBaseItem = DataBaseItem::new(data, custom_url.clone(), None);

    if let Some(seconds) = destroy {
        let seconds = String::from(String::from_utf8_lossy(seconds)).parse::<i64>();
        match seconds {
            Ok(seconds) => {
                item.destroy_time = Some(now + Duration::seconds(seconds));
            }
            Err(err) => {
                return Ok(warp::reply::with_status(
                    err.to_string(),
                    http::StatusCode::BAD_REQUEST,
                )
                .into_response());
            }
        }
    }
    let res = add_record(db.clone(), &item);
    let upload_status: UploadStatus;
    match res {
        Ok(_) => upload_status = UploadStatus::Created,
        Err(t) => match t {
            model::DataBaseErrorType::Existed(t) => {
                upload_status = UploadStatus::Existed;
                item = t;
            }
            model::DataBaseErrorType::Failed => upload_status = UploadStatus::Failed,
            model::DataBaseErrorType::NotFound => {
                unreachable!()
            }
        },
    }

    let response = UploadResponse {
        date: now.to_string(),
        digest: item.hash,
        size: content.unwrap().len(),
        status: upload_status,
        url: format!(
            "http://{}/{}",
            url,
            custom_url.unwrap_or(item.short.clone())
        ),
        short: item.short,
        uuid: item.uuid.to_string(),
    };
    info!(
        "{} {} of length {}",
        response.status.to_string(),
        response.short,
        response.size
    );
    Ok(warp::reply::with_status(response.to_string(), http::StatusCode::OK).into_response())
}

pub async fn upload(
    path: FullPath,
    form: FormData,
    db: model::DataTrees,
    url: String,
) -> Result<warp::reply::Response, Rejection> {
    process_upload(path, form, db, url, None).await
}

pub async fn custom_url_upload(
    custom_url: String,
    path: FullPath,
    form: FormData,
    db: model::DataTrees,
    url: String,
) -> Result<warp::reply::Response, Rejection> {
    process_upload(path, form, db, url, Some(custom_url)).await
}

pub async fn view_data(
    key: String,
    db: model::DataTrees,
) -> Result<warp::reply::Response, Rejection> {
    let mut database_key: String = key.clone().to_lowercase();
    let mut ext: String = String::from("txt");
    let mut has_ext = false;
    let now = Utc::now();
    if key.contains('.') {
        let res: Vec<&str> = key.split('.').collect();
        database_key = String::from(res[0]);
        ext = String::from(res[res.len() - 1]);
        has_ext = true;
    }
    if let Ok(data) = model::query_record(db.clone(), database_key.clone()) {
        info!("get {} success", key);
        if let Some(t) = data.destroy_time {
            if now > t {
                info!("... but it's expired");
                let delete_res = delete_record(db, data.uuid);
                match delete_res {
                    Ok(_) => {
                        log::info!("delete {} success", key);
                    }
                    Err(_) => {
                        log::warn!("delete {} key failed", key);
                    }
                }
                return Ok(warp::reply::with_status(
                    String::from("expired"),
                    http::StatusCode::BAD_REQUEST,
                )
                .into_response());
            }
        }
        match data.data {
            DataType::Text(c) => {
                log::info!(
                    "replying code {}",
                    c.chars().into_iter().take(10).collect::<String>()
                );
                if has_ext {
                    log::info!(
                        "highlighting code {}",
                        c.chars().into_iter().take(10).collect::<String>()
                    );
                    let html = highlight_lines(&c, &ext);
                    if let Some(html) = html {
                        return Ok(warp::reply::html(html).into_response());
                    }
                    log::warn!(
                        "highlight code {} with ext {} failed",
                        c.chars().into_iter().take(10).collect::<String>(),
                        ext
                    )
                }
                return Ok(warp::reply::with_status(c, http::StatusCode::OK).into_response());
            }
            DataType::ShortLink(l) => {
                log::info!("replying short link {}", l);
                let res = l.parse::<Uri>();
                match res {
                    Ok(t) => {
                        return Ok(warp::redirect(t).into_response());
                    }
                    Err(e) => {
                        return Ok(warp::reply::with_status(
                            e.to_string(),
                            http::StatusCode::BAD_REQUEST,
                        )
                        .into_response())
                    }
                }
            }
            DataType::Binary(t) => {
                //TODO: guess mime
                log::info!("serving binary");
                if has_ext {
                    log::info!("guessing mime type");
                    let guess = mime_guess::from_ext(ext.as_str());
                    let mime = guess.first();
                    return match mime {
                        Some(mime) => {
                            log::info!("guess {} as {}", ext, mime.to_string());
                            Ok(
                                warp::reply::with_header(t, "content-type", mime.to_string())
                                    .into_response(),
                            )
                        }
                        None => {
                            Ok(warp::reply::with_status(t, http::StatusCode::OK).into_response())
                        }
                    };
                }
                return Ok(warp::reply::with_status(t, http::StatusCode::OK).into_response());
            }
        }
    } else {
        info!("get {} failed", key);
        return Ok(warp::reply::with_status(
            String::from("not found"),
            http::StatusCode::NOT_FOUND,
        )
        .into_response());
    }
}

pub async fn delete_data(
    key: String,
    db: model::DataTrees,
) -> Result<warp::reply::Response, Rejection> {
    if let Ok(id) = uuid::Uuid::parse_str(key.as_str()) {
        let delete_res = delete_record(db, id);
        match delete_res {
            Ok(_) => {
                log::info!("delete {} success", key);
                return Ok(warp::reply::with_status(
                    format!("deleted {}", key),
                    http::StatusCode::OK,
                )
                .into_response());
            }
            Err(_) => {
                log::warn!("delete {} key failed", key);
            }
        }
    }
    Ok(
        warp::reply::with_status(format!("{} not found", key), http::StatusCode::NOT_FOUND)
            .into_response(),
    )
}

pub async fn update_data(
    key: String,
    db: model::DataTrees,
    host: String,
    form: FormData,
) -> Result<warp::reply::Response, Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
    })?;
    let data = read_multipart_form(parts).await;
    let content = data.get("c").or(data.get("content"));
    if let Ok(id) = uuid::Uuid::parse_str(key.as_str()) {
        if let Some(content) = content {
            let data = DataType::from_bytes(content.clone(), None).unwrap();
            let update_res = model::update_record(db.clone(), id, data);
            let item = model::query_record(db, key.clone()).unwrap();
            match update_res {
                Ok(_) => {
                    log::info!("update {} success", key);
                    return Ok(warp::reply::with_status(
                        format!("http://{}/{} updated", host, item.short),
                        http::StatusCode::OK,
                    )
                    .into_response());
                }
                Err(_) => {
                    log::warn!("update {} failed", key);
                }
            }
        }
    }
    Ok(
        warp::reply::with_status(format!("{} not found", key), http::StatusCode::BAD_REQUEST)
            .into_response(),
    )
}
