use poem::Endpoint;
use poem::EndpointExt;
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

const MAXIMUM_FILE_SIZE: usize = 1024 * 1024 * 1024; // 1G bytes
const MP4: &str = "video/mp4";
const MPEG: &str = "video/mpeg";

#[derive(Debug, ApiResponse)]
enum DownloadOkResponse {
    /// OK
    #[oai(status = 200, content_type = "video/mp4")]
    // Compiler error if replacing "video/mp4" with MP4
    MP4(Attachment<Vec<u8>>),
    /// OK
    #[oai(status = 200, content_type = "video/mpeg")]
    // Compiler error if replacing "video/mpeg" with MPEG
    Mpeg(Attachment<Vec<u8>>),
}

#[derive(Debug, ApiResponse)]
enum DownloadErrorResponse {
    /// File not found
    #[oai(status = 404)]
    NotFound,
    /// Internal Error
    #[oai(status = 999)]
    InternalError,
}

#[derive(Debug, ApiResponse)]
enum DeleteOkResponse {
    /// File was successfully removed
    #[oai(status = 204)]
    Success,
}

#[derive(Debug, ApiResponse)]
enum DeleteErrorResponse {
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

#[derive(Debug, ApiResponse)]
enum UploadOkResponse {
    /// File uploaded
    #[oai(status = 201)]
    Success(#[oai(header = "Location")] String),
}

#[derive(Debug, ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
enum UploadErrorResponse {
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
struct UploadPayload {
    data: Upload,
}

#[derive(Default, Debug, Object, Clone)]
pub struct File {
    pub content_type: String,
    pub name: String,
    pub size: usize,
    pub created_at: String,
    pub download_cnt: u64,
}

#[derive(Clone, Debug, Object)]
pub struct UploadedFile {
    pub fileid: String,
    /// filename
    pub name: String,
    /// file size(bytes)
    pub size: usize,
    /// Time when the data was saved on the server side
    pub created_at: String,
}

pub struct FilesApi {
    pub rwlock: RwLock<Resource>,
}

impl FilesApi {
    pub async fn new() -> Self {
        Self {
            rwlock: RwLock::new(Resource {
                files: HashMap::<uuid::Uuid, File>::new(),
                name: HashMap::<String, uuid::Uuid>::new(),
                storage: super::storage::Storage::new().await,
            }),
        }
    }
}

#[derive(Clone)]
pub struct Resource {
    pub files: HashMap<uuid::Uuid, File>,
    pub name: HashMap<String, uuid::Uuid>,
    pub storage: super::storage::Storage,
}

fn upload_middleware(ep: impl Endpoint) -> impl Endpoint {
    // File Size up to 1G bytes
    ep.with(poem::middleware::SizeLimit::new(MAXIMUM_FILE_SIZE))
}

#[OpenApi]
impl FilesApi {
    /// Download a video file by fileid. The file name will be restored as it was when you uploaded it.
    #[oai(path = "/files/:fileid", method = "get")]
    async fn download(
        &self,
        fileid: Path<String>,
    ) -> Result<DownloadOkResponse, DownloadErrorResponse> {
        // Check if valid uuid
        let id = uuid::Uuid::parse_str(&fileid.0).map_err(|err| {
            warn!(?err);
            DownloadErrorResponse::NotFound
        })?;
        let resource = self.rwlock.read().await;
        let (filename, content_type) = resource
            .files
            .get(&id)
            .map(|file| (file.name.clone(), file.content_type.clone()))
            .ok_or(DownloadErrorResponse::NotFound)?;
        // Retrieve file from storage
        let Some(data) = resource.storage.retrieve(&id).await else {
            return Err(DownloadErrorResponse::NotFound);
        };
        // Construct return attachment
        let attachment = Attachment::new(data)
            .attachment_type(AttachmentType::Attachment)
            .filename(&filename);
        let ret = match content_type.as_str() {
            MP4 => Ok(DownloadOkResponse::MP4(attachment)),
            MPEG => Ok(DownloadOkResponse::Mpeg(attachment)),
            // Unlikely path if 'upload' implementation is correct
            _ => Err(DownloadErrorResponse::InternalError),
        };
        drop(resource);

        // Download is ready so we +1 to download cnt
        if let Some(file) = self.rwlock.write().await.files.get_mut(&id) {
            file.download_cnt += 1;
        };
        ret
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
        let mut resource = self.rwlock.write().await;
        resource
            .files
            .remove(&id)
            .map(|file| {
                // Because file was deleted, the coresspoding name is also gone
                let _ = resource.name.remove(&file.name);
                DeleteOkResponse::Success
            })
            .ok_or(DeleteErrorResponse::NotFound)?;

        resource.storage.delete(&id).await;
        Ok(DeleteOkResponse::Success)
    }

    /// Upload a video file
    #[oai(path = "/files", method = "post", transform = "upload_middleware")]
    async fn upload(&self, upload: UploadPayload) -> Result<UploadOkResponse, UploadErrorResponse> {
        // Checking if empty file name
        let filename = upload
            .data
            .file_name()
            .ok_or(UploadErrorResponse::InternalError)?;
        // Checking if File already existed
        let mut resource = self.rwlock.write().await;
        if resource.name.contains_key(filename) {
            return Err(UploadErrorResponse::FileExists);
        }

        // Checking if expected content_type
        let content_type = upload
            .data
            .content_type()
            .filter(|&ty| matches!(ty, MP4 | MPEG))
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
        resource.storage.store(&id, &data).await;
        // Expensive locking to write
        /*
            Here we check again if File already existed
            Note that this checking is mandatory here even we've checked in the begining of this method
            It's unlikeyly to happen but still has a chance becaue that's the nature of multi-thread
        */
        if resource.name.contains_key(&filename) {
            return Err(UploadErrorResponse::FileExists);
        }
        // Create mapping between filename and uuid
        resource.name.insert(filename.to_string(), id);
        // Create mapping between uuid and file
        resource.files.insert(
            id,
            File {
                name: filename,
                size: data.len(),
                content_type,
                created_at,
                ..Default::default()
            },
        );

        Ok(UploadOkResponse::Success(format!("./storage/{}", id)))
    }

    /// List uploaded files
    #[oai(path = "/files", method = "get")]
    async fn list(&self) -> Json<Vec<UploadedFile>> {
        let resource = self.rwlock.read().await;
        // I feel it's more firendly for concurrency query that we clone the data and immediatly
        // release the lock rather than holding it until we finished constructing whole json returned value
        let cloned_files = resource.files.clone();
        drop(resource);

        // Simple mapping
        let vec = cloned_files
            .into_iter()
            .map(
                |(
                    id,
                    File {
                        name,
                        created_at,
                        size,
                        ..
                    },
                )| UploadedFile {
                    fileid: id.to_string(),
                    name,
                    size,
                    created_at,
                },
            )
            .collect::<Vec<_>>();

        Json(vec)
    }
    /// List top 10 downloaded files
    #[oai(path = "/files/top_10_download", method = "get")]
    async fn top_10_downloads(&self) -> Json<Vec<UploadedFile>> {
        let v = super::ext_feature::top_10_downloads(self).await;
        Json(v)
    }
}
