//! Git logic for RepoBell. Every Git call goes through the `git` command line.

pub mod discover;
mod git;
pub mod status;

pub use discover::discover;
pub use git::GitError;
pub use status::{status, RepoStatus};
