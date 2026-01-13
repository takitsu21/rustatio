use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

/// Embed the built UI files at compile time
/// The UI should be built to ../ui/dist before compiling the server
#[derive(Embed)]
#[folder = "../ui/dist"]
struct Assets;

/// Handler for serving static files
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact path first
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }

    // For SPA routing, serve index.html for non-asset paths
    if !path.contains('.') || path.is_empty() {
        if let Some(content) = Assets::get("index.html") {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(content.data.into_owned()))
                .unwrap();
        }
    }

    // 404 for everything else
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}
