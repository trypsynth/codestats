#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic, clippy::perf)]
#![deny(warnings)]
#![allow(
	clippy::multiple_crate_versions,
	reason = "gix and reqwest's transitive deps pull in duplicate versions we don't control"
)]

pub mod analysis;
pub mod config;
pub mod display;
pub mod langs;
