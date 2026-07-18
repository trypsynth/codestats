use std::{path::Path, sync::atomic::AtomicBool, time::Duration};

use crate::error::AppError;

const CLONE_TIMEOUT: Duration = Duration::from_secs(30);

/// Shallow-clone `url` into `dest`, which must already exist and be empty.
///
/// Only `http(s)` URLs are accepted; the clone is depth-1 and aborted after
/// [`CLONE_TIMEOUT`], both to bound how much work an arbitrary URL can trigger.
pub async fn clone(url: String, dest: &Path) -> Result<(), AppError> {
	if !(url.starts_with("http://") || url.starts_with("https://")) {
		return Err(AppError::BadRequest("git_url must start with http:// or https://".to_owned()));
	}
	let dest = dest.to_path_buf();
	let clone_task = tokio::task::spawn_blocking(move || {
		clone_blocking(&url, &dest).map_err(|err| AppError::BadRequest(err.to_string()))
	});
	match tokio::time::timeout(CLONE_TIMEOUT, clone_task).await {
		Ok(join_result) => join_result.map_err(|err| AppError::Internal(err.into()))?,
		Err(_elapsed) => Err(AppError::BadRequest(format!("Clone did not finish within {}s", CLONE_TIMEOUT.as_secs()))),
	}
}

fn clone_blocking(url: &str, dest: &Path) -> anyhow::Result<()> {
	let interrupt = AtomicBool::new(false);
	let mut prepare = gix::prepare_clone(url, dest)?
		.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(1.try_into().expect("1 is nonzero")));
	let (mut checkout, _fetch_outcome) = prepare.fetch_then_checkout(gix::progress::Discard, &interrupt)?;
	checkout.main_worktree(gix::progress::Discard, &interrupt)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn rejects_non_http_schemes() {
		let dest = tempfile::tempdir().unwrap();
		for url in ["file:///etc/passwd", "ftp://example.com/repo.git", "not a url"] {
			let err = clone(url.to_owned(), dest.path()).await.unwrap_err();
			assert!(matches!(err, AppError::BadRequest(msg) if msg.contains("http:// or https://")), "url: {url}");
		}
	}
}
