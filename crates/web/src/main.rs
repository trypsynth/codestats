#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic, clippy::perf)]
#![deny(warnings)]
#![allow(
	clippy::multiple_crate_versions,
	reason = "gix and reqwest's transitive deps pull in duplicate versions we don't control"
)]

mod error;
mod git_source;
mod zip_source;

use std::path::Path;

use axum::{
	Json, Router,
	extract::{DefaultBodyLimit, Multipart},
	http::header,
	response::{IntoResponse, Response},
	routing::{get, post},
};
use codestats::{
	analysis::CodeAnalyzer,
	config::AnalyzerConfig,
	display::{OutputFormat, ViewOptions, get_formatter},
};
use error::AppError;
use serde::Deserialize;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let addr = std::env::var("CODESTATS_WEB_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
	let app = Router::new()
		.route("/health", get(|| async { "ok" }))
		.route("/api/analyze/zip", post(analyze_zip).layer(DefaultBodyLimit::max(zip_source::MAX_UPLOAD_BYTES)))
		.route("/api/analyze/git", post(analyze_git));
	let listener = tokio::net::TcpListener::bind(&addr).await?;
	println!("Listening on http://{addr}");
	axum::serve(listener, app).await?;
	Ok(())
}

async fn analyze_zip(mut multipart: Multipart) -> Result<Response, AppError> {
	let Some(field) = multipart.next_field().await.map_err(|err| AppError::BadRequest(err.to_string()))? else {
		return Err(AppError::BadRequest("Expected a multipart field containing the zip file".to_owned()));
	};
	let bytes = field.bytes().await.map_err(|err| AppError::BadRequest(err.to_string()))?;
	let tmp = tempfile::tempdir()?;
	zip_source::extract(&bytes, tmp.path())?;
	analyze_dir(tmp.path()).await
}

#[derive(Deserialize)]
struct GitRequest {
	url: String,
}

async fn analyze_git(Json(payload): Json<GitRequest>) -> Result<Response, AppError> {
	let tmp = tempfile::tempdir()?;
	git_source::clone(payload.url, tmp.path()).await?;
	analyze_dir(tmp.path()).await
}

/// Run the analyzer against `dir` and return its JSON report as an HTTP response.
async fn analyze_dir(dir: &Path) -> Result<Response, AppError> {
	let dir = dir.to_path_buf();
	let json = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
		let analyzer = CodeAnalyzer::new(&dir, AnalyzerConfig::default());
		let results = analyzer.analyze()?;
		let formatter = get_formatter(OutputFormat::JsonCompact);
		let mut buf = Vec::new();
		formatter.write_output(&results, &dir, ViewOptions::default(), &mut buf)?;
		Ok(buf)
	})
	.await
	.map_err(anyhow::Error::from)??;
	Ok(([(header::CONTENT_TYPE, "application/json")], json).into_response())
}
