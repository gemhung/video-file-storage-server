//#![allow(unused)]
use std::collections::HashMap;

use poem::{error::BadRequest, listener::TcpListener, Result, Route, Server};
use poem_openapi::{
    param::Path,
    payload::{Attachment, AttachmentType, Json, PlainText},
    types::multipart::Upload,
    ApiResponse, Multipart, Object, OpenApi, OpenApiService,
};
use time::macros::*;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Object, Clone)]
struct File {
    content_type: Option<String>,
    filename: String,
    data: Vec<u8>,
    created_at: String,
}

#[derive(Debug, ApiResponse)]
enum HealthCheckResponse {
    /// OK
    #[oai(status = 200)]
    OK,
}

#[derive(Debug, ApiResponse)]
enum GetFileResponse {
    /// OK
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
enum DeleteFileResponse {
    /// File was successfully removed
    #[oai(status = 204)]
    Success,
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
enum UploadFileResponse {
    /// File uploaded
    #[oai(status = 201, content_type = "text/plain")]
    Success(PlainText<String>, #[oai(header = "Location")] String),
    /// Bad request
    #[oai(status = 400)]
    BadRequest(PlainText<String>),
    /// File exists
    #[oai(status = 409)]
    FileExists,
    /// Unsupported Media Type
    #[oai(status = 415)]
    UnsupportedMediaType,
    /// Internal Error
    #[oai(status = 999)]
    InternalError,
}

fn bad_request_handler(err: poem::Error) -> UploadFileResponse {
    tracing::info!(?err);
    UploadFileResponse::BadRequest(PlainText(err.to_string()))
}

#[derive(Debug, ApiResponse)]
enum ListFileResponse {
    /// File list
    #[oai(status = 200)]
    OK(Json<Vec<UploadedFile>>),
}

#[derive(Debug, Object)]
struct UploadedFile {
    fileid: String,
    /// filename
    name: String,
    /// file size(bytes)
    size: usize,
    /// Time when the data was saved on the server side
    created_at: String,
}

struct Status {
    files: HashMap<String, File>,
    name: HashMap<String, String>,
}

#[derive(Debug, Multipart)]
struct UploadPayload {
    data: Upload,
}

struct Api {
    status: RwLock<Status>,
}

#[OpenApi]
impl Api {
    /// Return the health of the service as HTTP 200 status. Useful to check if everything is configured correctly.
    #[oai(path = "/health", method = "get")]
    async fn health_check(&self) -> HealthCheckResponse {
        HealthCheckResponse::OK
    }

    /// Download a video file by fileid. The file name will be restored as it was when you uploaded it.
    #[oai(path = "/files/:fileid", method = "get")]
    async fn get(&self, fileid: Path<String>) -> GetFileResponse {
        let status = self.status.read().await;
        match status.files.get(&fileid.0) {
            Some(file) => {
                let mut attachment =
                    Attachment::new(file.data.clone()).attachment_type(AttachmentType::Attachment);
                //if let Some(filename) = &file.filename {
                    attachment = attachment.filename(&file.filename);
                //}
                GetFileResponse::Ok(attachment)
            }
            None => GetFileResponse::NotFound,
        }
    }

    /// Delete a video file
    #[oai(path = "/files/:fileid", method = "delete")]
    async fn delete(&self, fileid: Path<String>) -> DeleteFileResponse {
        let mut status = self.status.write().await;
        status
            .files
            .remove(&fileid.0)
            .map(|file| {
                status.name.remove(&file.filename);
                DeleteFileResponse::Success
            })
            .unwrap_or_else(|| DeleteFileResponse::NotFound)
    }

    /// Upload a video file
    #[oai(path = "/files", method = "post")]
    async fn upload(&self, upload: UploadPayload) -> UploadFileResponse {
        let Some(filename) = upload.data.file_name().map(ToString::to_string) else {
            return UploadFileResponse::InternalError;
        };

        match upload.data.content_type() {
            Some("video/mp4" | "video/mpeg") => {},
            _ => {
                return UploadFileResponse::UnsupportedMediaType;
            }
        }
        let mut status = self.status.write().await;
        if status.name.contains_key(&filename) {
            return UploadFileResponse::FileExists;
        }
        let id = Uuid::new_v4().to_string();
        let file = File {
            content_type: upload.data.content_type().map(ToString::to_string),
            filename:filename.clone(),
            data: upload.data.into_vec().await.map_err(BadRequest).unwrap(),
            created_at: now(),
        };
        status.files.insert(id.clone(), file);
        status.name.insert(filename, id);

        UploadFileResponse::Success(PlainText("".to_string()), "bucket1".to_string())
    }

    /// List uploaded files
    #[oai(path = "/files", method = "get")]
    async fn list(&self) -> ListFileResponse {
        let status = self.status.read().await;
        let vec = status
            .files
            .iter()
            .map(|(id, file)| UploadedFile {
                fileid: id.to_string(),
                name: file.filename.clone(),
                size: file.data.len(),
                created_at: file.created_at.clone(),
            })
            .collect::<Vec<_>>();

        ListFileResponse::OK(Json(vec))
    }
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .to_offset(offset!(+9)) // Japan time zone
        .to_string()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .init();

    let api_service = OpenApiService::new(
        Api {
            status: RwLock::new(Status {
                files: Default::default(),
                name: Default::default(),
            }),
        },
        "Video Storage Server API",
        "1.0",
    )
    .server("http://0.0.0.0:8080/v1");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run(Route::new().nest("/v1", api_service).nest("/", ui))
        .await
}
