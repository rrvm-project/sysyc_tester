use std::{convert::Infallible, path::PathBuf, process::Stdio, time::Duration};

use actix_web::{
    get,
    middleware::Logger,
    post,
    web::{self, Data},
    App, HttpServer, Responder,
};
use actix_web_lab::sse::{self, Event, Sse};
use clap::Parser;
use config::{read_config, Config};
use serde::Deserialize;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures::stream::{self, Stream};
use log::info;
use tokio_stream::StreamExt;

mod config;

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

#[derive(Deserialize)]
struct MyData {
    branch: String,
    commit_id: String,
}

#[post("/test")]
async fn test(
    data: web::Json<MyData>,
    state: web::Data<AppState>,
) -> std::io::Result<impl Responder> {
    let data = data.into_inner(); 
    let (branch, commit_hash) = (data.branch, data.commit_id);
    info!("branch: {}, commit_hash: {}", branch, commit_hash);
    
    let args = [
        &state.config.repo,
        &format!("{branch}"),
        &format!("./{commit_hash}"),
    ];

    let stream = command_output_stream(&args);

    let sse_stream = stream.map(|line| {
        Event::Data(sse::Data::new(line))
    }).map(Ok::<_, Infallible>); 
        
    let stream_response = Sse::from_stream(sse_stream).with_keep_alive(Duration::from_secs(1));

    // Spawn a task to wait for the child process to exit
    // tokio::spawn(async move {
    //     cmd.wait().await.unwrap();
    // });

    Ok(stream_response)
}

// 异步函数，创建子进程并将其输出转换为 Stream
fn command_output_stream(args: &[&String]) -> impl Stream<Item = String> {
    // 启动子进程
    let mut child = Command::new("./test.sh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    // 获取子进程的错误输出
    let stderr = child.stderr.take().expect("Failed to open stderr");
    // 获取子进程的标准输出
    let stdout = child.stdout.take().expect("Failed to open stdout");

    // 将标准输出转换为 BufReader
    let reader = BufReader::new(stderr);
    let reader2 = BufReader::new(stdout);

    // 将 BufReader 转换为一个异步 Stream
    let lines = reader.lines();
    let lines2 = reader2.lines();

    // 创建一个 Stream 来处理子进程的输出
    let stream1 = stream::unfold(lines, |mut lines| async {
        match lines.next_line().await {
            Ok(Some(line)) => Some((line, lines)),
            _ => None,
        }
    });
    let stream2 = stream::unfold(lines2, |mut lines| async {
        match lines.next_line().await {
            Ok(Some(line)) => Some((line, lines)),
            _ => None,
        }
    });

    stream1.merge(stream2)
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
    .bind(("0.0.0.0", 12345))?
    .run()
    .await
}
