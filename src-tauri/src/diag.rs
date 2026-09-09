//! In-app diagnostic log: the last few hundred lines of what the core did,
//! readable from Settings and copyable, because a release build has no console
//! and a phone in the field has no adb. Never log key material.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::core::now;

static LOG: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
const CAP: usize = 300;

pub fn log(line: impl AsRef<str>) {
    let line = line.as_ref();
    eprintln!("[encedo] {line}");
    if let Ok(mut l) = LOG.lock() {
        let t = now();
        let hms = format!("{:02}:{:02}:{:02}", (t / 3600) % 24, (t / 60) % 60, t % 60);
        l.push_back(format!("{hms} {line}"));
        while l.len() > CAP {
            l.pop_front();
        }
    }
}

pub fn lines() -> Vec<String> {
    LOG.lock().map(|l| l.iter().cloned().collect()).unwrap_or_default()
}

pub fn clear() {
    if let Ok(mut l) = LOG.lock() {
        l.clear();
    }
}

/// Shorten a long token-like string for the log.
pub fn brief(s: &str, keep: usize) -> String {
    if s.len() <= keep * 2 + 1 { s.to_string() } else { format!("{}…{}", &s[..keep], &s[s.len() - keep..]) }
}
