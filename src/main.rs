use poem::{listener::TcpListener, Result, Route, Server};
use poem_openapi::OpenApiService;
use tokio::sync::RwLock;

pub mod api;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .init();

    let api_service = OpenApiService::new(
        api::Api {
            status: RwLock::new(api::Status {
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
