use config::Config;
use std::sync::Arc;

use warp::Filter;
mod config;
mod controller;
mod highlighter;
mod model;
#[tokio::main]
async fn main() {
    highlighter::highlight_lines(&String::from(""),&String::from("rs"));
    flexi_logger::Logger::with_env_or_str("info")
        .format(flexi_logger::colored_default_format)
        .start()
        .unwrap();
    let sled_config = sled::Config::default()
        .cache_capacity(5_000_000)
        .use_compression(true)
        .path("db");
    let db: Arc<sled::Db> = Arc::new(sled_config.open().unwrap());
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
    // let shorten_url_route = warp::path("u")
    //     .and(warp::post())
    //     .and(warp::multipart::form().max_length(config.max_length))
    //     .and(db_filter.clone())
    //     .and(warp::header::<String>("host"))
    //     .and_then(controller::shorten_url);
    let route = upload_route.or(view_route);
    warp::serve(route).run(([127, 0, 0, 1], config.port)).await;
}
