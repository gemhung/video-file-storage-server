use poem_openapi::param::Path;
use poem_openapi::OpenApi;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub mod files;
pub mod health;

pub struct Api {
    pub status: RwLock<Status>,
}

pub struct Status {
    pub files: HashMap<String, files::File>,
    pub name: HashMap<String, String>,
}

#[OpenApi]
impl Api {
    /// Return the health of the service as HTTP 200 status. Useful to check if everything is configured correctly.
    #[oai(path = "/health", method = "get")]
    async fn health_check(&self) -> health::HealthCheckResponse {
        self.health_check_impl()
    }

    /// Download a video file by fileid. The file name will be restored as it was when you uploaded it.
    #[oai(path = "/files/:fileid", method = "get")]
    async fn download(&self, fileid: Path<String>) -> files::DownloadFileResponse {
        self.download_impl(fileid).await
    }

    /// Delete a video file
    #[oai(path = "/files/:fileid", method = "delete")]
    async fn delete(&self, fileid: Path<String>) -> files::DeleteFileResponse {
        self.delete_impl(fileid).await
    }

    /// Upload a video file
    #[oai(path = "/files", method = "post")]
    async fn upload(&self, upload: files::UploadPayload) -> files::UploadFileResponse {
        self.upload_impl(upload).await
    }

    /// List uploaded files
    #[oai(path = "/files", method = "get")]
    async fn list(&self) -> files::ListFileResponse {
        self.list_impl().await
    }
}
