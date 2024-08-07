// test 接口, 接受分支名和 commit hash 作为参数，拉取仓库，调用 test.sh 进行测试

use std::convert::Infallible;
use std::process::Stdio;
use std::time::Duration;

use actix_web::{post, web, Responder};
use actix_web_lab::sse::{self, Event, Sse};
use log::info;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use futures::stream::{self, Stream};

use crate::AppState;

#[derive(Deserialize)]
struct MyData {
    branch: String,
    commit_id: String,
}

#[post("/test")]
pub async fn test(
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