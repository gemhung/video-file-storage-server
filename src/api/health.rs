use poem_openapi::ApiResponse;
use poem_openapi::OpenApi;

#[derive(Debug, ApiResponse)]
pub enum HealthCheckResponse {
    /// OK
    #[oai(status = 200)]
    OK,
}

pub struct HealthApi;

#[OpenApi]
impl HealthApi {
    /// Return the health of the service as HTTP 200 status. Useful to check if everything is configured correctly.
    #[oai(path = "/health", method = "get")]
    async fn health_check(&self) -> HealthCheckResponse {
        HealthCheckResponse::OK
    }
}
