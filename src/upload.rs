// upload 接口，用于上传文件
use std::{collections::HashMap, fs, io::Write};

use actix_multipart::Multipart;
use actix_web::{
    post, web, Error, HttpResponse, Responder
};
use sanitize_filename::sanitize;
use tokio_stream::StreamExt;

#[post("/upload")]
pub async fn upload(mut payload: Multipart, query: web::Query<HashMap<String, String>>) -> Result<impl Responder, Error> {
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