//! The axum server behind `gym web`.
//!
//! The hydrated bundle is **inside the binary**: `build.rs` copies the files
//! `cargo leptos build` produced into `OUT_DIR` and this module embeds them, so
//! installing stays one file and there is no site directory to deploy. A build
//! without the web assets (a plain `cargo build`) still works as a CLI; `gym
//! web` then says so instead of serving an empty page.

use super::app::{shell, App};
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode, Uri},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, handle_server_fns, LeptosRoutes};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// Every response revalidates before it is reused.
///
/// The bundle's URLs are stable across builds, so a browser that pairs a cached
/// script with a freshly built WASM hydrates against a different DOM and panics
/// inside `tachys`. `no-cache` still allows a 304 — it forces the check rather
/// than disabling caching.
async fn no_cache(req: Request<Body>, next: middleware::Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache"),
    );
    res
}

async fn asset(uri: Uri) -> Response {
    let (body, kind): (Option<&'static [u8]>, &'static str) = match uri.path() {
        "/pkg/gym.js" => (JS, "text/javascript"),
        "/pkg/gym.wasm" | "/pkg/gym_bg.wasm" => (wasm(), "application/wasm"),
        "/pkg/gym.css" => (CSS, "text/css"),
        _ => (None, "text/plain"),
    };
    match body {
        Some(b) => (
            [
                (header::CONTENT_TYPE, kind),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            b,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn render(State(options): State<LeptosOptions>, uri: Uri) -> Response {
    let handler = leptos_axum::render_app_to_stream(move || shell(options.clone()));
    let req = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("a request");
    handler(req).await.into_response()
}

pub async fn run(addr: &str) -> Result<(), String> {
    if !embedded() {
        return Err(format!(
            "This build has no web UI: the hydrated bundle is not embedded. Rebuild with 'cargo leptos build --release' (or ./install.sh), then run 'gym web --addr {addr}' again."
        ));
    }
    // Built here rather than read from `Cargo.toml`: the installed binary has
    // no manifest beside it, and every field that matters is fixed at build
    // time anyway.
    let options = leptos::config::LeptosOptions::builder()
        .output_name("gym")
        .site_root("")
        .site_pkg_dir("pkg")
        .build();
    let routes = generate_route_list(App);
    let app = Router::new()
        .route("/pkg/gym.js", get(asset))
        .route("/pkg/gym.wasm", get(asset))
        .route("/pkg/gym_bg.wasm", get(asset))
        .route("/pkg/gym.css", get(asset))
        .route(
            "/api/{*fn_name}",
            get(handle_server_fns).post(handle_server_fns),
        )
        .leptos_routes_with_handler(routes, get(render))
        .layer(middleware::from_fn(no_cache))
        .with_state(options);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to '{addr}' because '{e}'."))?;
    println!("gym web on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| format!("The server stopped because '{e}'."))
}
