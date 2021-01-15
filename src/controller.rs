use std::{collections::HashMap, unreachable};

use bytes::BufMut;
use chrono::prelude::*;
use futures::TryStreamExt;
use log::info;

use model::{add_record, DataBaseItem, DataTrees};
use warp::{http, hyper::Uri, multipart::Part, path::FullPath};
use warp::{multipart::FormData, Buf};
use warp::{Rejection, Reply};

use crate::{
    highlighter::highlight_lines,
    model::{self, TextItem},
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

pub async fn upload(
    path: FullPath,
    form: FormData,
    db: model::DataTrees,
    url: String,
) -> Result<impl Reply, Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
    })?;
    let data = read_multipart_form(parts).await;
    let content = data.get("c").or(data.get("content"));
    let destroy = data.get("sunset");
    if let None = content {
        return Ok(warp::reply::with_status(
            String::from("error"),
            http::StatusCode::BAD_REQUEST,
        ));
    }
    let content = String::from(String::from_utf8_lossy(content.unwrap()));
    let mut item: DataBaseItem = DataBaseItem::new(
        if path.as_str() == "/u" {
            TextItem::ShortLink(String::from(content.trim_end()))
        } else {
            TextItem::Code(content.clone())
        },
        None, // TODO:
        None, // TODO:
    );
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
    let date: DateTime<Utc> = Utc::now();

    let response = UploadResponse {
        date: date.to_string(),
        digest: item.hash,
        size: content.len(),
        status: upload_status,
        url: format!("{}/{}", url, item.short),
        short: item.short,
        uuid: item.uuid.to_string(),
    };
    info!(
        "{} {} of length {}",
        response.status.to_string(),
        response.short,
        response.size
    );
    Ok(warp::reply::with_status(
        response.to_string(),
        http::StatusCode::OK,
    ))
}

pub async fn view_data(
    key: String,
    db: model::DataTrees,
) -> Result<warp::reply::Response, Rejection> {
    let mut database_key: String = key.clone().to_lowercase();
    let mut ext: String = String::from("txt");
    let mut highlighting = false;
    if key.contains('.') {
        let res: Vec<&str> = key.split('.').collect();
        database_key = String::from(res[0]);
        ext = String::from(res[res.len() - 1]);
        highlighting = true;
    }
    if let Ok(data) = model::query_record(db.clone(), database_key) {
        info!("get {} success", key);
        match data.text {
            TextItem::Code(c) => {
                log::info!("replying code");
                if highlighting {
                    log::info!("highlighting code");
                    let html = highlight_lines(&c, &ext);
                    return Ok(warp::reply::html(html).into_response());
                }
                return Ok(warp::reply::with_status(c, http::StatusCode::OK).into_response());
            }
            TextItem::ShortLink(l) => {
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
