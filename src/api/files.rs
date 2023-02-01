use poem_openapi::{
    param::Path,
    payload::{Attachment, AttachmentType, Json, PlainText},
    types::multipart::Upload,
    ApiResponse, Multipart, Object,
};
use time::format_description::FormatItem;
use time::macros::format_description;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, ApiResponse)]
pub enum DownloadFileResponse {
    /// OK
    #[oai(status = 200, content_type = "video/mp4")]
    MP4(Attachment<Vec<u8>>),
    #[oai(status = 200, content_type = "video/mpeg")]
    MPEG(Attachment<Vec<u8>>),
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
pub enum DeleteFileResponse {
    /// File was successfully removed
    #[oai(status = 204)]
    Success,
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
pub enum UploadFileResponse {
    /// File uploaded
    #[oai(status = 201)]
    Success(#[oai(header = "Location")] String),
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
    UploadFileResponse::BadRequest(PlainText(err.to_string()))
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

impl crate::api::Api {
    pub async fn download_impl(&self, fileid: Path<String>) -> DownloadFileResponse {
        let Ok(id) = uuid::Uuid::parse_str(&fileid.0) else {
            return DownloadFileResponse::NotFound;
        };
        let status = self.status.read().await;
        match status.files.get(&id) {
            None => DownloadFileResponse::NotFound,
            Some(file) => {
                let attachment =
                    Attachment::new(file.data.clone())
                    .attachment_type(AttachmentType::Attachment)
                    .filename(&file.filename);
                match file.content_type.as_str() {
                    "video/mp4" => DownloadFileResponse::MP4(attachment),
                    "video/mpeg" => DownloadFileResponse::MPEG(attachment),
                    _ => DownloadFileResponse::NotFound,
                }
            }
        }
    }

    pub async fn delete_impl(&self, fileid: Path<String>) -> DeleteFileResponse {
        // Invalid uuid is considered as not found
        let Ok(id) = uuid::Uuid::parse_str(&fileid.0) else {
            return DeleteFileResponse::NotFound;
        };
        // Try delete
        let mut status = self.status.write().await;
        status
            .files
            .remove(&id)
            .map(|file| {
                status.name.remove(&file.filename);
                DeleteFileResponse::Success
            })
            .unwrap_or_else(|| DeleteFileResponse::NotFound)
    }

    pub async fn upload_impl(&self, upload: UploadPayload) -> UploadFileResponse {
        // Checking if empty file name
        let Some(filename) = upload.data.file_name() else {
            return UploadFileResponse::InternalError;
        };

        // Checking if expected content_type
        let content_type = match upload.data.content_type() {
            Some(inner @ ("video/mp4" | "video/mpeg")) => inner.to_string(),
            _ => {
                return UploadFileResponse::UnsupportedMediaType;
            }
        };
        let filename = filename.to_string();
        // Extracting file data
        let data = match upload.data.into_vec().await {
            Ok(data) => data,
            Err(err) => {
                error!(?err);
                return UploadFileResponse::InternalError;
            }
        };
        let id = Uuid::new_v4();
        let created_at = now();
        // The idea is to minimize the duration of operations when locked
        let mut status = self.status.write().await;
        // Checking if File already existed
        if status.name.contains_key(&filename) {
            return UploadFileResponse::FileExists;
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

        UploadFileResponse::Success(format!("./mypath/{}", id))
    }

    pub async fn list_impl(&self) -> ListFileResponse {
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
