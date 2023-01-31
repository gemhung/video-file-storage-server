use poem::error::BadRequest;
use poem_openapi::{
    param::Path,
    payload::{Attachment, AttachmentType, Json, PlainText},
    types::multipart::Upload,
    ApiResponse, Multipart, Object,
};
use time::macros::*;
use uuid::Uuid;

#[derive(Debug, ApiResponse)]
pub enum DownloadFileResponse {
    /// OK
    #[oai(status = 200, content_type = "video/mp4")]
    OKMP4(Attachment<Vec<u8>>),
    #[oai(status = 200, content_type = "video/mpeg")]
    OKMPEG(Attachment<Vec<u8>>),
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
    #[oai(status = 201, header(name = "Location", type = "String"))]
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
    UploadFileResponse::BadRequest(PlainText(err.to_string()))
}

fn now() -> String {
    let format =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:9]Z");
    time::OffsetDateTime::now_utc()
        //.to_offset(offset!(+9)) // Japan time zone
        .format(&format)
        .unwrap()
}

#[derive(Debug, Multipart)]
pub struct UploadPayload {
    data: Upload,
}

#[derive(Debug, Object, Clone)]
pub struct File {
    pub content_type: Option<String>,
    pub filename: String,
    pub data: Vec<u8>,
    pub created_at: String,
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
        let status = self.status.read().await;
        match status.files.get(&fileid.0) {
            Some(file) => {
                let mut attachment =
                    Attachment::new(file.data.clone()).attachment_type(AttachmentType::Attachment);
                //if let Some(filename) = &file.filename {
                attachment = attachment.filename(&file.filename);
                //}
                match file.content_type.as_deref() {
                    Some("video/mp4") => DownloadFileResponse::OKMP4(attachment),
                    Some("video/mpeg") => DownloadFileResponse::OKMPEG(attachment),
                    _ => DownloadFileResponse::NotFound,
                }
            }
            None => DownloadFileResponse::NotFound,
        }
    }

    pub async fn delete_impl(&self, fileid: Path<String>) -> DeleteFileResponse {
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

    pub async fn upload_impl(&self, upload: UploadPayload) -> UploadFileResponse {
        let Some(filename) = upload.data.file_name().map(ToString::to_string) else {
            return UploadFileResponse::InternalError;
        };

        match upload.data.content_type() {
            Some("video/mp4" | "video/mpeg") => {}
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
            filename: filename.clone(),
            data: upload.data.into_vec().await.map_err(BadRequest).unwrap(),
            created_at: now(),
        };
        let location = format!("./mypath/{}", id);
        status.files.insert(id.clone(), file);
        //status.name.insert(filename, id);

        UploadFileResponse::Success(PlainText("".to_string()), location)
        //UploadFileResponse::Success("bucket1".to_string())
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
