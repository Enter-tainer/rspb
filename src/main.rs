use config::Config;
use model::DataTrees;

use warp::Filter;
mod base32;
mod config;
mod controller;
mod highlighter;
mod markdown;
mod model;
#[tokio::main]
async fn main() {
    let config: Config = config::Config::load(None).await.unwrap_or_default();
    let help = markdown::render(
        tokio::fs::read_to_string("README.md")
            .await
            .unwrap_or(String::from("cmd | curl -F c=@- https://pb.mgt.moe/")),
    );
    highlighter::highlight_lines(&String::from(""), &String::from("rs"));
    flexi_logger::Logger::with_env_or_str("info")
        .format(flexi_logger::colored_default_format)
        .start()
        .unwrap();
    let sled_config = sled::Config::default()
        .cache_capacity(config.db_cache_capacity)
        .use_compression(true)
        .path("db");
    let db: sled::Db = sled_config.open().unwrap();
    let model: model::DataTrees = DataTrees::new(db);
    let model_filter = warp::any().map(move || model.clone());
    let help_route = warp::path::end()
        .and(warp::get())
        .map(move || warp::reply::html(help.clone()));
    let upload_route = warp::path::end()
        .or(warp::path("u"))
        .unify()
        .and(warp::path::full())
        .and(warp::post())
        .and(warp::multipart::form().max_length(config.max_length))
        .and(model_filter.clone())
        .and(warp::header::<String>("host"))
        .and_then(controller::upload);
    let custom_url_route = warp::post()
        .and(warp::path!(String))
        .and(warp::path::full())
        .and(warp::multipart::form().max_length(config.max_length))
        .and(model_filter.clone())
        .and(warp::header::<String>("host"))
        .and_then(controller::custom_url_upload);
    let view_route = warp::get()
        .and(warp::path!(String))
        .and(model_filter.clone())
        .and_then(controller::view_data);
    let delete_route = warp::delete()
        .and(warp::path!(String))
        .and(model_filter.clone())
        .and_then(controller::delete_data);
    let update_route = warp::put()
        .and(warp::path!(String))
        .and(model_filter.clone())
        .and(warp::header::<String>("host"))
        .and(warp::multipart::form().max_length(config.max_length))
        .and_then(controller::update_data);

    let route = upload_route
        .or(view_route)
        .or(delete_route)
        .or(custom_url_route)
        .or(update_route)
        .or(help_route);
    warp::serve(route).run((config.ip, config.port)).await;
}
