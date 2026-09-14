//! Whether this build may still be used.
//!
//! Play answers three things about a newer release: its version code, the
//! priority the publisher set on it (0-5) and how long this phone has been
//! behind. It carries no words, so what was wrong lives in the release notes and
//! the app's own screen sends the person there to read them.
//!
//! Two rules matter more than the arithmetic:
//!
//! - A phone that cannot reach Play is not a phone that may keep running a
//!   version known to be dangerous. The verdict is remembered next to the store,
//!   in the clear (it is not a secret, and the screen has to work before the
//!   storage is open), and it only ever rises.
//! - A phone that cannot reach Play is also not one to lock out on suspicion. No
//!   answer plus nothing remembered means the app carries on.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Priority at or above which a release is treated as one nobody may skip. Set
/// it on the release in the Play Console; 5 is Google's most urgent.
const CRITICAL_PRIORITY: i32 = 4;
/// Falling this far behind is worth a word even when the release was routine.
const STALE_DAYS: i64 = 14;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Nothing to say.
    #[default]
    None,
    /// Worth updating; the app keeps working.
    Recommended,
    /// This build must not be used: the app answers nothing until it is updated.
    Critical,
}

/// What Play said, as the app needs it. Mirrors the plugin's answer so this
/// module builds and is tested everywhere, not only on a phone.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayAnswer {
    pub available: bool,
    pub version_code: i64,
    pub priority: i32,
    pub stale_days: i64,
    /// Play can replace this build in place; false for a sideloaded APK.
    pub can_update_in_app: bool,
}

/// What the screens are given.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateStatus {
    pub level: Level,
    /// The version Play is offering, or the one this phone was told to reach.
    pub required_version: i64,
    pub current_version: i64,
    pub priority: i32,
    pub stale_days: i64,
    pub can_update_in_app: bool,
    pub checked_at: u64,
    /// Why Play could not be asked, when it could not.
    pub note: Option<String>,
    /// This verdict was pretended in a development build. The blocking screen
    /// hides every other screen, including the one with the button that started
    /// the pretence, so it has to offer its own way out.
    pub pretended: bool,
}

/// The verdict kept between launches, so cutting the network does not lift it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Remembered {
    /// The lowest version this phone may run.
    #[serde(default)]
    pub required_version: i64,
    #[serde(default)]
    pub level: Level,
    #[serde(default)]
    pub since: u64,
    #[serde(default)]
    pub pretended: bool,
}

/// What Play's answer means for a phone running `current`.
pub fn decide(play: &PlayAnswer, current: i64) -> Level {
    if !play.available || play.version_code <= current {
        return Level::None;
    }
    if play.priority >= CRITICAL_PRIORITY {
        return Level::Critical;
    }
    if play.priority >= 1 || play.stale_days >= STALE_DAYS {
        return Level::Recommended;
    }
    Level::None
}

pub fn load(path: &Path) -> Remembered {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save(path: &Path, remembered: &Remembered) {
    if let Ok(bytes) = serde_json::to_vec(remembered) {
        let _ = std::fs::write(path, bytes);
    }
}

pub fn forget(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Fold what Play said into what this phone already knew. Returns the status to
/// show and whether the verdict changed, which is what earns a line in the
/// journal.
pub fn settle(path: &Path, play: &PlayAnswer, current: i64, now: u64, pretending: bool) -> (UpdateStatus, bool) {
    let mut known = load(path);
    // Reached the version it was told to reach: nothing left to remember.
    if known.required_version > 0 && current >= known.required_version {
        forget(path);
        known = Remembered::default();
    }

    let fresh = decide(play, current);
    let remembered = if known.required_version > current { known.level } else { Level::None };
    let level = fresh.max(remembered);

    let required_version = match level {
        Level::None => 0,
        _ => play.version_code.max(known.required_version),
    };

    // Once pretended, always pretended: a real check later must not take the way
    // out away from a phone that is only playing.
    let pretended = pretending || known.pretended;
    let changed = level != known.level || required_version > known.required_version || pretended != known.pretended;
    if changed {
        match level {
            Level::None => forget(path),
            _ => save(path, &Remembered { required_version, level, since: now, pretended }),
        }
    }

    let status = UpdateStatus {
        level,
        required_version,
        current_version: current,
        priority: play.priority,
        stale_days: play.stale_days,
        can_update_in_app: play.can_update_in_app,
        checked_at: now,
        note: None,
        pretended,
    };
    (status, changed)
}

impl Level {
    /// Ordering for `max`: nothing < recommended < critical.
    fn rank(self) -> u8 {
        match self {
            Level::None => 0,
            Level::Recommended => 1,
            Level::Critical => 2,
        }
    }

    fn max(self, other: Level) -> Level {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }

    pub fn is_blocking(self) -> bool {
        self == Level::Critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(available: bool, version: i64, priority: i32, stale: i64) -> PlayAnswer {
        PlayAnswer { available, version_code: version, priority, stale_days: stale, can_update_in_app: true }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("encedo-update-{}-{name}.json", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn priority_decides_how_loudly_to_ask() {
        assert_eq!(decide(&play(false, 0, 0, 0), 10), Level::None);
        assert_eq!(decide(&play(true, 11, 0, 0), 10), Level::None, "a routine release nobody is behind on says nothing");
        assert_eq!(decide(&play(true, 11, 1, 0), 10), Level::Recommended);
        assert_eq!(decide(&play(true, 11, 0, 14), 10), Level::Recommended, "two weeks behind is worth a word");
        assert_eq!(decide(&play(true, 11, 4, 0), 10), Level::Critical);
        assert_eq!(decide(&play(true, 11, 5, 0), 10), Level::Critical);
        // Play sometimes offers what is already installed.
        assert_eq!(decide(&play(true, 10, 5, 0), 10), Level::None);
    }

    #[test]
    fn a_blocking_verdict_survives_the_network_being_cut() {
        let path = temp("offline");
        let (status, changed) = settle(&path, &play(true, 20, 5, 0), 10, 1_000, false);
        assert_eq!((status.level, status.required_version, changed), (Level::Critical, 20, true));

        // Play cannot be reached at all next time; the phone still knows.
        let (status, changed) = settle(&path, &play(false, 0, 0, 0), 10, 2_000, false);
        assert_eq!(status.level, Level::Critical);
        assert_eq!(status.required_version, 20);
        assert!(!changed, "nothing new to write down");

        // And it is lifted by the only thing that should lift it.
        let (status, changed) = settle(&path, &play(false, 0, 0, 0), 20, 3_000, false);
        assert_eq!((status.level, changed), (Level::None, false));
        assert!(!path.exists(), "the phone stops carrying a verdict it has satisfied");
    }

    #[test]
    fn a_pretended_verdict_stays_marked_as_one() {
        let path = temp("pretend");
        let (status, _) = settle(&path, &play(true, 20, 5, 0), 10, 1_000, true);
        assert!(status.pretended, "the screen has to know it may offer a way out");

        // A real check later must not quietly take that way out away.
        let (status, _) = settle(&path, &play(true, 21, 5, 0), 10, 2_000, false);
        assert!(status.pretended);
        assert!(load(&path).pretended);

        // Leaving the pretence is the same thing as satisfying the verdict.
        forget(&path);
        let (status, _) = settle(&path, &play(false, 0, 0, 0), 10, 3_000, false);
        assert_eq!((status.level, status.pretended), (Level::None, false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_verdict_rises_but_never_falls_on_its_own() {
        let path = temp("rising");
        settle(&path, &play(true, 20, 2, 0), 10, 1_000, false);
        assert_eq!(load(&path).level, Level::Recommended);

        // The same release turns out to be urgent after all.
        let (status, changed) = settle(&path, &play(true, 20, 5, 0), 10, 2_000, false);
        assert_eq!((status.level, changed), (Level::Critical, true));

        // A later routine release must not quietly undo that.
        let (status, _) = settle(&path, &play(true, 21, 0, 0), 10, 3_000, false);
        assert_eq!(status.level, Level::Critical);
        assert_eq!(status.required_version, 21, "and it names the newest version that fixes it");
        let _ = std::fs::remove_file(&path);
    }
}
