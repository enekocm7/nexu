use axum::{Router, routing::get};
use std::path::PathBuf;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct MediaServer {
    port: u16,
}

impl MediaServer {
    pub fn new(port: u16) -> Self {
        MediaServer { port }
    }

    pub async fn start(&self, media_path: PathBuf) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/", get("Hello World!!"))
            .nest_service("/media", ServeDir::new(media_path));

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        println!("Media server started on http://{}", addr);

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("Server error: {}", e);
            }
        });

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
