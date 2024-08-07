// compile 接口，用于将源代码编译成汇编代码，在这之前应当通过 upload 上传源代码
use actix_web::{
    post, web, Error, HttpResponse, Responder
};
use tokio::process::Command;

use crate::run::{FilesToRun, RunResult};

#[post("/compile")]
pub async fn compile(data: web::Json<FilesToRun>) -> Result<impl Responder, Error> {
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