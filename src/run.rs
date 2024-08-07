// run 接口，用于运行汇编代码，在这之前应当将汇编代码和它的输入输出通过 upload 上传
use std::{fs::File, io::BufRead, process::Stdio, time::Instant};

use actix_web::{
    post, web, Error, HttpResponse, Responder
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Deserialize)]
pub struct FilesToRun {
    pub folder: String,
    pub name: String,
    pub name_without_suffix: String,
}

#[derive(Serialize, Deserialize)]
pub struct RunResult{
    pub code: u32,
    pub time: f64,
}

fn get_answer(file: &str) -> (Vec<String>, i32) {
    let file = File::open(file).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    let answer_exitcode = lines.last().unwrap().parse::<i32>().unwrap();
    let answer_content = lines[..lines.len()-1].to_vec();

    (answer_content, answer_exitcode)
}


#[post("/run")]
pub async fn run(data: web::Json<FilesToRun>) -> Result<impl Responder, Error> {
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