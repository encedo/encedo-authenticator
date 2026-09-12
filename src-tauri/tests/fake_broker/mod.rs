//! A broker on 127.0.0.1 for the core tests: answers are scripted per route,
//! every request the app sent is kept for inspection. Speaks just enough
//! HTTP/1.1 for reqwest, including keep-alive, so no extra crate is needed.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One request as it arrived.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: String,
}

impl Recorded {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("{} {} body is not json ({e}): {}", self.method, self.path, self.body))
    }
}

struct Rule {
    method: String,
    path: String,
    /// Answers in order; the last one repeats once the queue runs down.
    replies: VecDeque<(u16, String)>,
}

#[derive(Default)]
struct State {
    rules: Vec<Rule>,
    seen: Vec<Recorded>,
}

impl State {
    fn answer(&mut self, method: &str, path: &str) -> (u16, String) {
        for r in self.rules.iter_mut() {
            if r.method == method && path.contains(&r.path) {
                if r.replies.len() > 1 {
                    return r.replies.pop_front().expect("non-empty");
                }
                return r.replies.front().cloned().expect("non-empty");
            }
        }
        // Not a teapot on purpose: 418 maps to `Rejected`, which carries the body
        // into the error message, so an unscripted call names itself in the failure.
        (418, format!("no rule for {method} {path}"))
    }
}

pub struct FakeBroker {
    pub base: String,
    state: Arc<Mutex<State>>,
    task: tokio::task::JoinHandle<()>,
}

impl FakeBroker {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let state = Arc::new(Mutex::new(State::default()));
        let task = tokio::spawn({
            let state = state.clone();
            async move {
                while let Ok((sock, _)) = listener.accept().await {
                    tokio::spawn(serve(sock, state.clone()));
                }
            }
        });
        Self { base, state, task }
    }

    /// Answer `method` on any path containing `path` with `status` and `body`.
    /// Calling it again for the same route queues the next answer.
    pub fn on(&self, method: &str, path: &str, status: u16, body: &str) -> &Self {
        let mut s = self.state.lock().expect("state");
        if let Some(r) = s.rules.iter_mut().find(|r| r.method == method && r.path == path) {
            r.replies.push_back((status, body.to_string()));
        } else {
            s.rules.push(Rule { method: method.into(), path: path.into(), replies: VecDeque::from(vec![(status, body.to_string())]) });
        }
        self
    }

    /// Answer `method` on `path` with this and only this, whatever was queued
    /// before.
    pub fn only(&self, method: &str, path: &str, status: u16, body: &str) -> &Self {
        self.state.lock().expect("state").rules.retain(|r| !(r.method == method && r.path == path));
        self.on(method, path, status, body)
    }

    pub fn seen(&self) -> Vec<Recorded> {
        self.state.lock().expect("state").seen.clone()
    }

    pub fn count(&self, method: &str, path: &str) -> usize {
        self.seen().iter().filter(|r| r.method == method && r.path.contains(path)).count()
    }

    /// The last matching request; panics when the app never made it.
    pub fn last(&self, method: &str, path: &str) -> Recorded {
        self.seen()
            .into_iter()
            .filter(|r| r.method == method && r.path.contains(path))
            .next_back()
            .unwrap_or_else(|| panic!("the app never sent {method} …{path}"))
    }
}

impl Drop for FakeBroker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(mut sock: TcpStream, state: Arc<Mutex<State>>) {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        // Headers, then as much body as Content-Length promises.
        let head_end = loop {
            if let Some(i) = find(&buf, b"\r\n\r\n") {
                break i + 4;
            }
            if !read_more(&mut sock, &mut buf).await {
                return;
            }
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let mut first = head.lines().next().unwrap_or_default().split_whitespace();
        let method = first.next().unwrap_or_default().to_string();
        let path = first.next().unwrap_or_default().to_string();
        let len: usize = head
            .lines()
            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0)))
            .unwrap_or(0);
        while buf.len() < head_end + len {
            if !read_more(&mut sock, &mut buf).await {
                return;
            }
        }
        let body = String::from_utf8_lossy(&buf[head_end..head_end + len]).to_string();
        buf.drain(..head_end + len);

        let (status, reply) = {
            let mut s = state.lock().expect("state");
            s.seen.push(Recorded { method: method.clone(), path: path.clone(), body });
            s.answer(&method, &path)
        };
        let resp = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{reply}",
            reply.len()
        );
        if sock.write_all(resp.as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn read_more(sock: &mut TcpStream, buf: &mut Vec<u8>) -> bool {
    let mut chunk = [0u8; 4096];
    match sock.read(&mut chunk).await {
        Ok(0) | Err(_) => false,
        Ok(n) => {
            buf.extend_from_slice(&chunk[..n]);
            true
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
