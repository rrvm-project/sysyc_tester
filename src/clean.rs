use std::{collections::HashMap, fs};

// clean 接口，清除存放上传文件的临时文件夹
use actix_web::{
    post, web, Error, HttpResponse, Responder
};
use sanitize_filename::sanitize;


#[post("/clean")]
pub async fn clean(query: web::Query<HashMap<String, String>>) -> Result<impl Responder, Error> {
    let folder_name = query.get("folder").ok_or_else(|| {
        actix_web::error::ErrorBadRequest("Folder name is missing")
    })?;
    let folder_path = format!("./uploaded_files/{}", sanitize(folder_name.as_str()));
    fs::remove_dir_all(&folder_path)?;
    Ok(HttpResponse::Ok().body("Folder cleaned"))
}