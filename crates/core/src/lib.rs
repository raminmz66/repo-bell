//! Git logic for RepoBell. Every Git call goes through the `git` command line.

pub mod discover;
pub mod errors;
pub mod feed;
pub mod fetch;
mod git;
pub mod status;
pub mod update;

pub use discover::discover;
pub use errors::{classify, Problem};
pub use feed::{feed, remote_main_sha, Change, Feed};
pub use fetch::fetch_main;
pub use git::GitError;
pub use status::{status, RepoStatus};
pub use update::{can_undo, plan, undo, update, Blocked, Outcome, Plan, UndoOutcome, UndoPoint};
