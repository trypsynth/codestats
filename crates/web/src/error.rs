use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde::Serialize;

/// Errors that can escape a request handler, mapped to an HTTP status.
#[derive(Debug)]
pub enum AppError {
	/// The client sent something we can point at as wrong: a malformed zip, a
	/// disallowed URL scheme, a zip entry that would escape the extraction dir.
	BadRequest(String),
	/// Everything else (I/O failures, clone failures, analysis failures).
	Internal(anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
	error: String,
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, message) = match self {
			Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
			Self::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
		};
		(status, Json(ErrorBody { error: message })).into_response()
	}
}

impl From<anyhow::Error> for AppError {
	fn from(err: anyhow::Error) -> Self {
		Self::Internal(err)
	}
}

impl From<std::io::Error> for AppError {
	fn from(err: std::io::Error) -> Self {
		Self::Internal(err.into())
	}
}
