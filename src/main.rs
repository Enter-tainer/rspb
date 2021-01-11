use std::{sync::Arc};
use config::Config;

use warp::{Filter};
mod config;
mod controller;
#[tokio::main]
async fn main() {
    flexi_logger::Logger::with_env_or_str("info").format(flexi_logger::colored_default_format).start().unwrap();
    let db: Arc<sled::Db> = Arc::new(sled::open("db").unwrap());
    let config: Config = config::Config::load(None).await.unwrap_or_default();
    let db_filter = warp::any().map(move || db.clone());
    let upload_route = warp::path::end()
        .and(warp::post())
        .and(warp::multipart::form().max_length(config.max_length))
        .and(db_filter.clone())
        .and(warp::header::<String>("host"))
        .and_then(controller::upload);
    let view_route = warp::path!(String)
        .and(warp::get())
        .and(db_filter.clone())
        .and_then(controller::view_data);
    let route = upload_route.or(view_route);
    warp::serve(route).run(([127, 0, 0, 1], config.port)).await;
}
