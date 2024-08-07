use std::path::PathBuf;

use actix_web::{
    get, middleware::Logger, web::{self, Data}, App, HttpServer, Responder
};
use clap::Parser;
use config::{read_config, Config};

mod config;
mod run;
mod test;
mod upload;
mod clean;
mod compile;

#[derive(Parser, Clone)]
#[clap(author="cyh2004", version="0.1.0", about="", long_about=None)]
struct Cli {
    #[clap(short, long, value_parser, default_value = "./config.json")]
    config: PathBuf,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    cli: Cli,
    config: Config,
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello, {}!", name)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cli = Cli::parse();
    let config = read_config(&cli.config);
    let state = AppState { cli, config };

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .wrap(Logger::default())
            .service(greet)
            .service(test::test)
            .service(upload::upload)
            .service(run::run)
            .service(compile::compile)
            .service(clean::clean)
    })
    .bind(("0.0.0.0", 12345))?
    .run()
    .await
}
