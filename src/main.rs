use std::error::Error;
use std::future::ready;
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::body::{boxed, Body};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{middleware, Extension, Router, Server};

use tower_http::trace::TraceLayer;
use tracing::info;

mod app_tracing;
mod fake_datastore;
mod metrics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    app_tracing::setup()?;
    run_server().await?;
    app_tracing::teardown();
    Ok(())
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:3779".parse()?;
    info!("Listening on http://{}", addr);

    let recorder_handle = metrics::setup_metrics_recorder();

    let app = Router::new()
        .route(
            "/repos/mirror/:plat/:repo/:arch/:snapshot/*path",
            get(resolve_static_file_request),
        )
        .route("/health", get(health))
        .route("/metrics", get(move || ready(recorder_handle.render())))
        .route_layer(middleware::from_fn(metrics::track_metrics))
        .layer(Extension(fake_datastore::get_repository_map()))
        .layer(TraceLayer::new_for_http());

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument(skip(mapping))]
async fn resolve_static_file_request(
    Path((platform, repo, arch, snapshot, path)): Path<(String, String, String, String, String)>,
    Extension(mapping): Extension<&'static fake_datastore::RepoMap>,
) -> Response {
    let repo_name = format!("{platform}-{repo}-{arch}");

    if let Some(definition) = mapping.lookup_by_name(&repo_name) {
        if path == "/repodata/repomd.xml" {
            info!("Requested repomd file at '{path}' for snapshot '{snapshot}', serving from disk");
            get_repomd(&repo_name, &snapshot).await.into_response()
        } else if path == format!("/{repo_name}.repo").as_str() {
            info!("Requested .repo file at '{path}', generating...");
            generate_repo_file(&platform, &repo_name, &arch, &snapshot).into_response()
        } else {
            let snapshot_url = {
                match definition.snapshots.get(&snapshot) {
                    Some(snapshot_url) => snapshot_url,
                    None => return StatusCode::NOT_FOUND.into_response(),
                }
            };
            let redirect_url = format!("{}{}", snapshot_url, &path[1..]);
            info!(
                "Requested file at '{}', redirecting to '{}'",
                path, &redirect_url
            );
            Redirect::to(&redirect_url).into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[tracing::instrument]
async fn get_repomd(repo_name: &str, snapshot: &str) -> impl IntoResponse {
    // on-disk path to the repomd.xml to serve
    let repomd_path = PathBuf::from(format!("snapshots/{repo_name}/{snapshot}")).join("repomd.xml");
    info!("Serving file '{}'", repomd_path.to_str().unwrap());
    // read repomd.xml to String, build response
    match tokio::fs::read_to_string(&repomd_path).await {
        Err(_) => StatusCode::NOT_FOUND.into_response(),
        Ok(repomd_content) => Response::builder()
            .status(StatusCode::OK)
            .body(boxed(Body::from(repomd_content)))
            .unwrap(),
    }
}

#[tracing::instrument]
fn generate_repo_file(platform: &str, repo: &str, arch: &str, snapshot: &str) -> impl IntoResponse {
    // TODO: dynamic hostname
    format!(
        r"[{repo}]
name={repo}
enabled=1
baseurl=http://localhost:3779/{platform}/{repo}/{arch}/{snapshot}/
gpgcheck=1
repo_gpgcheck=0")
}

#[tracing::instrument]
async fn health() -> impl IntoResponse {
    StatusCode::OK
}
