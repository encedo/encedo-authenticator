//! `encedo-auth://pair?qr=…` links the system hands to the app. A link can
//! arrive before the webview is ready, while the app is locked or while the
//! person is still on the Welcome screen, so it waits here until the webview
//! asks for it. Only the link is kept; reading it is `PairingCode::parse`.

use std::sync::Mutex;

use crate::notify::PAIR_SCHEME;

#[derive(Default)]
pub struct PairLinks {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    /// The newest link not yet handed to the webview.
    pending: Option<String>,
    /// The last one handed over. Android keeps answering "the link this app
    /// was started with" for the life of the process, so it is not taken twice.
    last: Option<String>,
}

impl PairLinks {
    /// Keep `url` for the webview. False when it is not this app's scheme or
    /// is the link already waiting or already handed over.
    pub fn offer(&self, url: &str) -> bool {
        let ours = url.split_once(':').is_some_and(|(s, _)| s.eq_ignore_ascii_case(PAIR_SCHEME));
        let Ok(mut i) = self.inner.lock() else { return false };
        if !ours || i.pending.as_deref() == Some(url) || i.last.as_deref() == Some(url) {
            return false;
        }
        i.pending = Some(url.to_string());
        true
    }

    /// The waiting link, once.
    pub fn take(&self) -> Option<String> {
        let mut i = self.inner.lock().ok()?;
        let url = i.pending.take()?;
        i.last = Some(url.clone());
        Some(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_is_handed_over_once() {
        let l = PairLinks::default();
        assert!(!l.offer("https://example.org/"));
        assert!(l.offer("encedo-auth://pair?qr=a"));
        assert!(!l.offer("encedo-auth://pair?qr=a"));
        assert_eq!(l.take().as_deref(), Some("encedo-auth://pair?qr=a"));
        assert_eq!(l.take(), None);
        assert!(!l.offer("encedo-auth://pair?qr=a"), "the start link again");
        // A newer link replaces one not yet taken.
        assert!(l.offer("ENCEDO-AUTH://pair?qr=b"));
        assert!(l.offer("encedo-auth://pair?qr=c"));
        assert_eq!(l.take().as_deref(), Some("encedo-auth://pair?qr=c"));
    }
}
