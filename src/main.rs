use std::{collections::HashMap, convert::Infallible, path::PathBuf, time::Duration};

use actix_web::{
    get,
    middleware::Logger,
    post,
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use actix_web_lab::sse;
use clap::Parser;
use config::{read_config, Config};
use log::info;
use utils::run_command;

mod config;
mod utils;

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
    // log::info!("Greeted {}", name);
    format!("Hello, {}!", name)
}

#[get("/from-stream")]
async fn from_stream() -> impl Responder {
    let event_stream =
        futures_util::stream::iter([Ok::<_, Infallible>(sse::Event::Data(sse::Data::new("foo")))]);

    sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(5))
}

#[post("/test/{branch}/{commit_hash}")]
async fn test(
    path_args: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> std::io::Result<impl Responder> {
    let (branch, commit_hash) = path_args.into_inner();
    info!("branch: {}, commit_hash: {}", branch, commit_hash);

    let args = [
        "clone",
        "-b",
        &format!("{branch}"),
        &state.config.repo,
        &format!("./{commit_hash}"),
    ];
    run_command("git", &args, ".")?;

    run_command(
        "git",
        &["checkout", &commit_hash],
        &format!("./{commit_hash}"),
    )?;

    run_command(
        "git",
        &["submodule", "update", "--init", "--recursive"],
        &format!("./{commit_hash}"),
    )?;

    run_command(
        "cargo",
        &["build", "--workspace", "--release"],
        &format!("./{commit_hash}"),
    )?;

    run_command(
        "make",
        &[],
        &format!("./{commit_hash}/project-eval/runtime"),
    )?;

    let func_output = run_command(
        "python",
        &["test.py", "-t", "./testcases/functional", "-b"],
        &format!("./{commit_hash}/project-eval"),
    )?;

    let perf_output = run_command(
        "python",
        &["test.py", "-t", "./testcases/performance", "-b"],
        &format!("./{commit_hash}/project-eval"),
    )?;

    let mut res = HashMap::new();
    res.insert("func_output", func_output.stdout);
    res.insert("perf_output", perf_output.stdout);

    Ok(HttpResponse::Ok().json(&res))
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
            .service(from_stream)
            .service(test)
    })
    .bind(("127.0.0.1", 12345))?
    .run()
    .await
}
