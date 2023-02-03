use poem_openapi::OpenApi;
use poem_openapi::{
    param::Path,
    payload::{Attachment, AttachmentType, Json, PlainText},
    types::multipart::Upload,
    ApiResponse, Multipart, Object,
};
use std::collections::HashMap;
use time::format_description::FormatItem;
use time::macros::format_description;
use tokio::sync::RwLock;
use tracing::error;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, ApiResponse)]
pub enum DownloadOkResponse {
    /// OK
    #[oai(status = 200, content_type = "video/mp4")]
    MP4(Attachment<Vec<u8>>),
    #[oai(status = 200, content_type = "video/mpeg")]
    Mpeg(Attachment<Vec<u8>>),
}

#[derive(Debug, ApiResponse)]
pub enum DownloadErrorResponse {
    /// File not found
    #[oai(status = 404)]
    NotFound,
    /// Internal Error
    #[oai(status = 999)]
    InternalError,
}

#[derive(Debug, ApiResponse)]
pub enum DeleteOkResponse {
    /// File was successfully removed
    #[oai(status = 204)]
    Success,
}

#[derive(Debug, ApiResponse)]
pub enum DeleteErrorResponse {
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
pub enum UploadOkResponse {
    /// File uploaded
    #[oai(status = 201)]
    Success(#[oai(header = "Location")] String),
}

#[derive(Debug, ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
pub enum UploadErrorResponse {
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

fn bad_request_handler(err: poem::Error) -> UploadErrorResponse {
    UploadErrorResponse::BadRequest(PlainText(err.to_string()))
}

// ISO 8601 time format
const TIME_FORMAT_8601: &[FormatItem] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:9]Z");

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&TIME_FORMAT_8601)
        .unwrap()
}

#[derive(Debug, Multipart)]
pub struct UploadPayload {
    data: Upload,
}

#[derive(Debug, Object, Clone)]
pub struct File {
    content_type: String,
    filename: String,
    data: Vec<u8>,
    created_at: String,
}

#[derive(Debug, Object)]
pub struct UploadedFile {
    fileid: String,
    /// filename
    name: String,
    /// file size(bytes)
    size: usize,
    /// Time when the data was saved on the server side
    created_at: String,
}
#[derive(Debug, ApiResponse)]
pub enum ListFileResponse {
    /// File list
    #[oai(status = 200)]
    OK(Json<Vec<UploadedFile>>),
}

pub struct FilesApi {
    pub status: RwLock<Status>,
}

pub struct Status {
    pub files: HashMap<uuid::Uuid, File>,
    pub name: HashMap<String, uuid::Uuid>,
}

#[OpenApi]
impl FilesApi {
    /// Download a video file by fileid. The file name will be restored as it was when you uploaded it.
    #[oai(path = "/files/:fileid", method = "get")]
    //async fn download(&self, fileid: Path<String>) -> DownloadFileResponse {
    async fn download(
        &self,
        fileid: Path<String>,
    ) -> Result<DownloadOkResponse, DownloadErrorResponse> {
        // Check if valid uuid
        let id = uuid::Uuid::parse_str(&fileid.0).map_err(|err| {
            warn!(?err);
            DownloadErrorResponse::NotFound
        })?;
        let status = self.status.read().await;
        let file = status
            .files
            .get(&id)
            .ok_or(DownloadErrorResponse::NotFound)?;

        // Read file from "./storage"
        let read_path = std::path::Path::new("./storage").join(id.to_string());
        let data = tokio::fs::read(read_path).await.map_err(|err| {
            error!(?err);
            DownloadErrorResponse::InternalError
        })?;

        let attachment = Attachment::new(data)
            .attachment_type(AttachmentType::Attachment)
            .filename(&file.filename);
        match file.content_type.as_str() {
            "video/mp4" => Ok(DownloadOkResponse::MP4(attachment)),
            "video/mpeg" => Ok(DownloadOkResponse::Mpeg(attachment)),
            _ => Err(DownloadErrorResponse::NotFound),
        }
    }

    /// Delete a video file
    #[oai(path = "/files/:fileid", method = "delete")]
    async fn delete(&self, fileid: Path<String>) -> Result<DeleteOkResponse, DeleteErrorResponse> {
        // We defined it's 'NotFound' when the fileid can't be converted into uuid
        let id = uuid::Uuid::parse_str(&fileid.0).map_err(|err| {
            warn!(?err);
            DeleteErrorResponse::NotFound
        })?;

        // Lock to delete
        let mut status = self.status.write().await;
        status
            .files
            .remove(&id)
            .map(|file| {
                // Because file was deleted, the coresspoding name is also gone
                let _ = status.name.remove(&file.filename);
                DeleteOkResponse::Success
            })
            .ok_or(DeleteErrorResponse::NotFound)
    }

    /// Upload a video file
    #[oai(path = "/files", method = "post")]
    async fn upload(&self, upload: UploadPayload) -> Result<UploadOkResponse, UploadErrorResponse> {
        // Checking if empty file name
        let filename = upload
            .data
            .file_name()
            .ok_or(UploadErrorResponse::InternalError)?;

        // Checking if expected content_type
        let content_type = upload
            .data
            .content_type()
            .filter(|ty| matches!(ty, &"video/mp4" | &"video/mpeg"))
            .ok_or(UploadErrorResponse::UnsupportedMediaType)?
            .to_string();
        let filename = filename.to_string();
        let data = upload.data.into_vec().await.map_err(|err| {
            error!(?err);
            UploadErrorResponse::InternalError
        })?;
        // Save file data to local storage
        let id = Uuid::new_v4();
        let created_at = now();
        let path = std::path::Path::new("./storage").join(id.to_string());
        tokio::fs::write(path, data.clone()).await.map_err(|err| {
            error!(?err);
            UploadErrorResponse::InternalError
        })?;

        // When locking, the idea is to minimize the duration of operations
        // to shorten the locking time
        let mut status = self.status.write().await;
        // Checking if File already existed
        if status.name.contains_key(&filename) {
            return Err(UploadErrorResponse::FileExists);
        }
        // Create mapping between filename and uuid
        status.name.insert(filename.clone(), id);
        // Create mapping between uuid and file
        let file = File {
            filename,
            content_type,
            data,
            created_at,
        };
        status.files.insert(id, file);
        drop(status); // release lock

        Ok(UploadOkResponse::Success(format!("./mypath/{}", id)))
    }

    /// List uploaded files
    #[oai(path = "/files", method = "get")]
    async fn list(&self) -> Json<Vec<UploadedFile>> {
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

        Json(vec)
    }
}
