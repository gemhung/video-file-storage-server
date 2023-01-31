use poem_openapi::ApiResponse;

#[derive(Debug, ApiResponse)]
pub enum HealthCheckResponse {
    /// OK
    #[oai(status = 200)]
    OK,
}

impl crate::api::Api {
    pub fn health_check_impl(&self) -> HealthCheckResponse {
        HealthCheckResponse::OK
    }
}
