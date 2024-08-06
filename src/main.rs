use std::{collections::HashMap, convert::Infallible, fs::{self, File}, io::{BufRead, Write}, path::PathBuf, process::Stdio, time::{Duration, Instant}};

use actix_multipart::Multipart;
use actix_web::{
    get, middleware::Logger, post, web::{self, Data}, App, Error, HttpResponse, HttpServer, Responder
};
use actix_web_lab::sse::{self, Event, Sse};
use clap::Parser;
use config::{read_config, Config};
use futures::stream::{self, Stream};
use log::info;
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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
    format!("Hello, {}!", name)
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
        &format!("{commit_hash}"),
    ];

    let stream = command_output_stream(&args);

    let sse_stream = stream
        .map(|line| Event::Data(sse::Data::new(line)))
        .map(Ok::<_, Infallible>);

    let stream_response = Sse::from_stream(sse_stream).with_keep_alive(Duration::from_secs(1));

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
    let stderr_lines = BufReader::new(stderr).lines();
    let stdout_lines = BufReader::new(stdout).lines();

    // 将 BufReader 的 Lines 转换为异步 Stream
    let stream1 = stream::unfold(stderr_lines, |mut lines| async {
        match lines.next_line().await {
            Ok(Some(line)) => Some((line, lines)),
            _ => None,
        }
    });
    let stream2 = stream::unfold(stdout_lines, |mut lines| async {
        match lines.next_line().await {
            Ok(Some(line)) => Some((line, lines)),
            _ => None,
        }
    });

    stream1.merge(stream2)
}

#[post("/upload")]
async fn upload(mut payload: Multipart, query: web::Query<HashMap<String, String>>) -> Result<impl Responder, Error> {
    let folder_name = query.get("folder").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Folder name is missing")
    })?;
    while let Some(mut field) = payload.try_next().await? {
        let folder_path = format!("./uploaded_files/{}", sanitize(folder_name.as_str()));

        // Create the folder if it does not exist
        fs::create_dir_all(&folder_path)?;

        // Generate a unique file name or use the original file name
        let content_disposition = field.content_disposition().unwrap();
        let filename = content_disposition.get_filename().unwrap();
        let filepath = format!("{}/{}", folder_path, sanitize(filename));

        // Create a new file
        let mut f = web::block(|| std::fs::File::create(filepath)).await??;

        // Write file content to the new file
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || f.write_all(&data).map(|_| f)).await??;
        }
    }
    Ok(HttpResponse::Ok().body("File uploaded"))
}

#[derive(Deserialize)]
struct FilesToRun {
    folder: String,
    name: String,
    name_without_suffix: String,
}

fn get_answer(file: &str) -> (Vec<String>, i32) {
    let file = File::open(file).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    let answer_exitcode = lines.last().unwrap().parse::<i32>().unwrap();
    let answer_content = lines[..lines.len()-1].to_vec();

    (answer_content, answer_exitcode)
}

#[derive(Serialize, Deserialize)]
struct RunResult{
    code: u32,
    time: f64,
}

#[post("/run")]
async fn run(data: web::Json<FilesToRun>) -> Result<impl Responder, Error> {
    let base_name = "uploaded_files/".to_string() + &data.folder + "/" + &data.name;
    let base_name_without_suffix = "uploaded_files/".to_string() + &data.folder + "/" + &data.name_without_suffix;
    let assembly =  base_name.clone() + ".s";
    let executable = base_name.clone() + ".exec";
    let output = base_name.clone() + ".stdout";
    let outerr = base_name.clone() + ".stderr";
    let answer = base_name_without_suffix.clone() + ".out";
    let input = base_name_without_suffix.clone() + ".in";
    let compile_status = Command::new("gcc")
        .args(&["-march=rv64gc", "-mabi=lp64d", &assembly, "runtime/libsysy.a", "-o", &executable])
        .status()
        .await?;
    if !compile_status.success() {
        return Ok(HttpResponse::BadRequest().json(RunResult{ code: 1, time: 0.0 })); // 1: Linker error 
    }
    let (answer_content, answer_exitcode) = get_answer(&answer);    
    let start_time = Instant::now();

    let command = Command::new(executable)
        .stdin(if let Ok(v) = File::open(input) { Stdio::from(v) } else { Stdio::null() })
        .stdout(Stdio::from(File::create(output.clone()).unwrap()))
        .stderr(Stdio::from(File::create(outerr).unwrap()))
        .status().await?;

    let end_time = Instant::now();
    let output_content: Vec<String> = std::io::BufReader::new(File::open(output.clone()).unwrap())
        .lines()
        .map(|line| line.unwrap())
        .collect();

    if command.code() != Some(answer_exitcode)
        || output_content != answer_content
    {
        return Ok(HttpResponse::BadRequest().json(RunResult{ code: 2, time: 0.0 })); // 2: Wrong answer
    }

    let duration = end_time.duration_since(start_time).as_secs_f64() * 1000.0;
    Ok(HttpResponse::Ok().json(RunResult{ code: 0, time: duration })) // 0: Success
}

#[post("/compile")]
async fn compile(data: web::Json<FilesToRun>) -> Result<impl Responder, Error> {
    let base_name = "uploaded_files/".to_string() + &data.folder + "/" + &data.name;
    let assembly =  base_name.clone() + "-gcc.s";
    let source = base_name.clone() + ".sy";
    let compile_status = Command::new("gcc")
        .args(&["-xc++", "-O2", "-S", "-march=rv64gc", "-mabi=lp64d", "-include", "runtime/sylib.h", &source, "-o", &assembly])
        .status()
        .await?;
    if !compile_status.success() {
        return Ok(HttpResponse::BadRequest().json(RunResult{ code: 3, time: 0.0 })); // 3: Gcc error 
    }
    Ok(HttpResponse::Ok().json(RunResult{ code: 0, time: 0.0 })) // 0: Success
}

#[post("/clean")]
async fn clean(query: web::Query<HashMap<String, String>>) -> Result<impl Responder, Error> {
    let folder_name = query.get("folder").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Folder name is missing")
    })?;
    let folder_path = format!("./uploaded_files/{}", sanitize(folder_name.as_str()));
    fs::remove_dir_all(&folder_path)?;
    Ok(HttpResponse::Ok().body("Folder cleaned"))
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
            .service(test)
            .service(upload)
            .service(run)
            .service(compile)
            .service(clean)
    })
    .bind(("0.0.0.0", 12345))?
    .run()
    .await
}
