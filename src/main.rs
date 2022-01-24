mod webhook;
mod bot;

#[macro_use] extern crate log;
use std::collections::HashMap;
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use config::Config;
use lazy_static::lazy_static;
use std::sync::RwLock;
use actix_web::web::Data;
use crate::bot::test_api;
use crate::webhook::{Webhook, handle};

lazy_static! {
    static ref SETTINGS: RwLock<Config> = RwLock::new(init_config());
}

#[derive(Debug)]
pub struct AppState {
    webhooks: HashMap<String, Webhook>
}

fn init_config() -> Config {
    let mut settings = Config::new();
    settings.merge(config::File::with_name("config.toml"))
        .unwrap()
        .merge(config::Environment::with_prefix("BOT"))
        .unwrap();

    settings
}

#[get("/")]
async fn heartbeat() -> impl Responder {
    HttpResponse::Ok().body("success")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_level = match SETTINGS.read().unwrap().get_bool("debug").unwrap_or(false) {
        false => "qq_gitlab_bot",
        true => match SETTINGS.read().unwrap().get_bool("verbose").unwrap_or(false) {
            false => "info",
            true => "true",
        },
    };

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, log_level));

    if let Some(about) = test_api().await {
        info!("Successfully connected to onebot api: {}, {}, protocol {}", about.app_name, about.app_version, about.protocol);
    }

    let listen = SETTINGS.read().unwrap().get::<String>("common.listen")
        .unwrap_or(String::from("127.0.0.1:5800"));
    info!("Start listening on {}", listen);

    HttpServer::new(|| {
        App::new()
            .service(handle)
            .service(heartbeat)
            .app_data(Data::new(AppState {
                webhooks: SETTINGS.read().unwrap().get::<HashMap<String, Webhook>>("webhook").unwrap()
            }))
    })
        .bind(listen)?
        .run()
        .await
}
