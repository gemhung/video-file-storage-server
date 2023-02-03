use poem::{listener::TcpListener, Result, Route, Server};
use poem_openapi::OpenApiService;

mod api;

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
    .server("http://0.0.0.0:8080/v1");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    // Start listening
    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run(
            Route::new()
                .nest("/v1", api_service)
                .nest("/", ui)
                .at("/spec", spec)
                .at("/spec_yaml", spec_yaml),
        )
        .await
}
