//! Embedded web dist (feature `embedded-web`): serves `web/dist` at `/` so the
//! server binary is a single deployable artifact. Same-origin WS/REST — no CORS.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// Compile-time embed of the Vite output. `web/dist` must exist at `cargo
/// build` time (`pixi run web-build` first); missing dist fails the build
/// loudly via the folder() macro requirement.
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/dist"]
struct Assets;

fn serve(path: &str) -> Response {
    match Assets::get(path) {
        Some(f) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                f.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

pub async fn index() -> Response {
    serve("index.html")
}

pub async fn asset(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    serve(&path)
}
