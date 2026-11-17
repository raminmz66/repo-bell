//! In-app updates: the state the window and tray read, and the rules that change it.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::utils::config::BundleType;
use tauri_plugin_updater::Error;

/// 1 MB is 1,000,000 bytes, here and in the window.
pub const MB: u64 = 1_000_000;

const CHECK_EVERY: u64 = 24 * 3600;
const RETRY_AFTER: u64 = 3600;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Ready,
    Downloading,
    Installing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateError {
    Offline,
    DownloadFailed,
    BadDownload,
    NotInstalled,
}

#[derive(Default)]
pub struct UpdateState {
    pub current: String,
    pub can_update: bool,
    pub deb: bool,
    pub status: UpdateStatus,
    /// Unix seconds of the last check that reached GitHub.
    pub checked_at: Option<u64>,
    /// When the last check ran and whether it worked. Only the timer reads it.
    pub last_try: Option<(u64, bool)>,
    /// The update found, kept for Install.
    pub pending: Option<tauri_plugin_updater::Update>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<UpdateError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Available {
    pub version: String,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateView {
    pub current: String,
    pub can_update: bool,
    pub deb: bool,
    pub status: UpdateStatus,
    pub checked_at: Option<u64>,
    pub available: Option<Available>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<UpdateError>,
    /// A repo action is running, so Install has to wait.
    pub busy: bool,
}

impl UpdateState {
    /// Only a .deb or an AppImage can replace itself; `cargo run` and a plain build can't.
    pub fn for_bundle(current: String, bundle: Option<BundleType>) -> Self {
        Self {
            current,
            can_update: matches!(bundle, Some(BundleType::Deb | BundleType::AppImage)),
            deb: bundle == Some(BundleType::Deb),
            ..Self::default()
        }
    }

    pub fn view(&self, actions_running: u32) -> UpdateView {
        UpdateView {
            current: self.current.clone(),
            can_update: self.can_update,
            deb: self.deb,
            status: self.status,
            checked_at: self.checked_at,
            available: self.pending.as_ref().map(|u| Available {
                version: u.version.clone(),
                notes: u.body.clone().unwrap_or_default(),
            }),
            downloaded: self.downloaded,
            total: self.total,
            error: self.error,
            busy: actions_running > 0,
        }
    }

    /// Starts a check. `None` when one can't run now; else the status to go back to if it fails.
    pub fn begin_check(&mut self) -> Option<UpdateStatus> {
        if !self.can_update || !matches!(self.status, UpdateStatus::Idle | UpdateStatus::UpToDate) {
            return None;
        }
        let before = self.status;
        self.status = UpdateStatus::Checking;
        self.error = None;
        Some(before)
    }

    pub fn finish_check(
        &mut self,
        found: Result<Option<tauri_plugin_updater::Update>, Error>,
        before: UpdateStatus,
        manual: bool,
        now: u64,
    ) {
        self.last_try = Some((now, found.is_ok()));
        match found {
            Ok(Some(update)) => {
                self.pending = Some(update);
                self.status = UpdateStatus::Ready;
                self.checked_at = Some(now);
            }
            Ok(None) => {
                self.status = UpdateStatus::UpToDate;
                self.checked_at = Some(now);
            }
            // Every check error counts as offline; only a check the user started says so.
            Err(_) => {
                self.status = before;
                self.error = manual.then_some(UpdateError::Offline);
            }
        }
    }

    /// Both guards run under the caller's state lock, so a second press can't start a second
    /// download.
    pub fn start_install(&mut self, actions_running: u32) -> Result<(), String> {
        if actions_running > 0 {
            return Err("Wait for the repo action to finish.".into());
        }
        if self.status != UpdateStatus::Ready {
            return Err("There's no update to install.".into());
        }
        self.status = UpdateStatus::Downloading;
        self.error = None;
        self.downloaded = 0;
        self.total = None;
        Ok(())
    }

    /// Repo actions may not start while the app is about to restart.
    pub fn action_allowed(&self) -> Result<(), String> {
        match self.status {
            UpdateStatus::Downloading | UpdateStatus::Installing => {
                Err("RepoBell is installing an update and will restart in a moment.".into())
            }
            _ => Ok(()),
        }
    }
}

/// How long the timer sleeps before it looks again.
pub fn wait_for(setup_done: bool, u: &UpdateState, now: u64) -> Duration {
    if !setup_done || !matches!(u.status, UpdateStatus::Idle | UpdateStatus::UpToDate) {
        return Duration::from_secs(60);
    }
    let due = match u.last_try {
        None => return Duration::ZERO,
        Some((t, true)) => t.saturating_add(CHECK_EVERY),
        Some((t, false)) => t.saturating_add(RETRY_AFTER),
    };
    Duration::from_secs(due.saturating_sub(now))
}

/// What the window says about a failed download (`installing` false) or install.
pub fn error_for(e: &Error, installing: bool) -> UpdateError {
    match (e, installing) {
        (
            Error::Minisign(_)
            | Error::Base64(_)
            | Error::SignatureUtf8(_)
            | Error::SignedVersionMismatch { .. }
            | Error::MissingSignedVersion,
            _,
        )
        | (Error::InvalidUpdaterFormat, true) => UpdateError::BadDownload,
        (_, false) => UpdateError::DownloadFailed,
        (_, true) => UpdateError::NotInstalled,
    }
}

/// True when the downloaded size crossed into a new whole MB, so a bar update is worth sending.
pub fn mb_changed(before: u64, after: u64) -> bool {
    before == 0 || before / MB != after / MB
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tauri::utils::config::BundleType;
    use tauri_plugin_updater::Error;

    fn io() -> Error {
        Error::Io(std::io::Error::other("x"))
    }

    fn can() -> UpdateState {
        UpdateState {
            can_update: true,
            ..UpdateState::default()
        }
    }

    #[test]
    fn bundle_type_decides_can_update() {
        let s = UpdateState::for_bundle("0.1.0".into(), Some(BundleType::Deb));
        assert!(s.can_update && s.deb);
        let s = UpdateState::for_bundle("0.1.0".into(), Some(BundleType::AppImage));
        assert!(s.can_update && !s.deb);
        let s = UpdateState::for_bundle("0.1.0".into(), None);
        assert!(!s.can_update);
        assert_eq!(tauri::utils::platform::bundle_type(), None);
    }

    #[test]
    fn wait_for_follows_the_rules() {
        let now = 100_000;
        let idle = UpdateState::default();
        assert_eq!(wait_for(false, &idle, now), Duration::from_secs(60));
        let ready = UpdateState {
            status: UpdateStatus::Ready,
            ..UpdateState::default()
        };
        assert_eq!(wait_for(true, &ready, now), Duration::from_secs(60));
        assert_eq!(wait_for(true, &idle, now), Duration::ZERO);
        let ok = UpdateState {
            last_try: Some((now - 3 * 3600, true)),
            ..UpdateState::default()
        };
        assert_eq!(wait_for(true, &ok, now), Duration::from_secs(21 * 3600));
        let failed = UpdateState {
            last_try: Some((now - 600, false)),
            ..UpdateState::default()
        };
        assert_eq!(wait_for(true, &failed, now), Duration::from_secs(50 * 60));
    }

    #[test]
    fn error_mapping_matches_the_table() {
        let sig = || Error::SignatureUtf8("x".into());
        assert_eq!(error_for(&sig(), false), UpdateError::BadDownload);
        assert_eq!(error_for(&io(), false), UpdateError::DownloadFailed);
        assert_eq!(
            error_for(&Error::InvalidUpdaterFormat, true),
            UpdateError::BadDownload
        );
        assert_eq!(
            error_for(&Error::PackageInstallFailed, true),
            UpdateError::NotInstalled
        );
        assert_eq!(error_for(&io(), true), UpdateError::NotInstalled);
    }

    #[test]
    fn failed_auto_check_keeps_status_and_shows_nothing() {
        let mut s = UpdateState {
            status: UpdateStatus::UpToDate,
            checked_at: Some(50),
            ..can()
        };
        assert_eq!(s.begin_check(), Some(UpdateStatus::UpToDate));
        assert_eq!(s.status, UpdateStatus::Checking);
        s.finish_check(Err(io()), UpdateStatus::UpToDate, false, 200);
        assert_eq!(s.status, UpdateStatus::UpToDate);
        assert_eq!(s.error, None);
        assert_eq!(s.checked_at, Some(50));
        assert_eq!(s.last_try, Some((200, false)));
    }

    #[test]
    fn failed_manual_check_says_offline() {
        let mut s = can();
        let before = s.begin_check().unwrap();
        s.finish_check(Err(io()), before, true, 200);
        assert_eq!(s.status, UpdateStatus::Idle);
        assert_eq!(s.error, Some(UpdateError::Offline));
    }

    #[test]
    fn check_without_update_is_up_to_date() {
        let mut s = can();
        s.finish_check(Ok(None), UpdateStatus::Idle, true, 200);
        assert_eq!(s.status, UpdateStatus::UpToDate);
        assert_eq!(s.checked_at, Some(200));
        assert_eq!(s.last_try, Some((200, true)));
    }

    #[test]
    fn no_check_while_busy_or_not_updatable() {
        let mut s = can();
        assert_eq!(s.begin_check(), Some(UpdateStatus::Idle));
        assert_eq!(s.begin_check(), None);
        assert_eq!(UpdateState::default().begin_check(), None);
    }

    #[test]
    fn install_guard() {
        let mut s = UpdateState {
            status: UpdateStatus::Ready,
            ..can()
        };
        assert_eq!(
            s.start_install(1),
            Err("Wait for the repo action to finish.".to_string())
        );
        assert_eq!(s.status, UpdateStatus::Ready);
        assert_eq!(s.start_install(0), Ok(()));
        assert_eq!(s.status, UpdateStatus::Downloading);
        assert_eq!(s.error, None);
        assert!(s.start_install(0).is_err());
        let msg = "RepoBell is installing an update and will restart in a moment.";
        assert_eq!(s.action_allowed(), Err(msg.to_string()));
        s.status = UpdateStatus::Installing;
        assert_eq!(s.action_allowed(), Err(msg.to_string()));
        s.status = UpdateStatus::Ready;
        s.error = Some(UpdateError::NotInstalled);
        assert_eq!(s.action_allowed(), Ok(()));
    }

    #[test]
    fn mb_step_publishes_on_whole_mb() {
        assert!(mb_changed(0, 500));
        assert!(!mb_changed(500, 900_000));
        assert!(mb_changed(900_000, 1_000_100));
        assert!(!mb_changed(1_000_100, 1_999_999));
        assert!(mb_changed(1_999_999, 2_000_000));
    }
}
