//! Git logic for RepoBell. Every Git call goes through the `git` command line.

pub mod discover;
pub mod feed;
pub mod fetch;
mod git;
pub mod status;

pub use discover::discover;
pub use feed::{feed, remote_main_sha, Change, Feed};
pub use fetch::fetch_main;
pub use git::GitError;
pub use status::{status, RepoStatus};
