use poem::{listener::TcpListener, Result, Route, Server};
use poem_openapi::OpenApiService;

use poem::{middleware::TowerLayerCompatExt, EndpointExt};
use tower::limit::RateLimitLayer;
mod api;

const HOST: &str = "0.0.0.0:8080";
const VERSION: &str = "v1";
const HTTP: &str = "http://";

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Log init
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .init();

    // It's where we save the upload file
    tokio::fs::create_dir_all("./storage").await?;

    // Openapi service
    let api_service = OpenApiService::new(
        (
            api::health::HealthApi,
            api::files::FilesApi {
                ..Default::default()
            },
        ),
        "Video Storage Server API",
        "1.0",
    )
    .server(HTTP.to_string() + HOST + "/" + VERSION); // http://0.0.0.0:8080/v1
                                                      // Helper routes. can remove for production release
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    // Rate limit up to 1000 req in 30 seconds
    let api_service =
        api_service.with(RateLimitLayer::new(1000, std::time::Duration::from_secs(30)).compat());

    // Start listening
    Server::new(TcpListener::bind(HOST))
        .run(
            Route::new()
                .nest("/v1", api_service)
                .nest("/", ui)
                .at("/spec", spec)
                .at("/spec_yaml", spec_yaml),
        )
        .await
}
