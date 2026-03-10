use crate::http::response::HttpResponse;
use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

pub struct StaticFiles {
    root: PathBuf,
}

impl StaticFiles {
    pub fn new(root: &str) -> Self {
        StaticFiles {
            root: PathBuf::from(root),
        }
    }

    pub async fn serve(&self, request_path: &str) -> Option<HttpResponse> {
        let file_path = self.resolve_path(request_path)?;

        let bytes = ReadFileFuture::new(file_path.clone()).await.ok()?;

        let content_type = infer_content_type(&file_path);

        Some(HttpResponse::ok(bytes, content_type))
    }

    fn resolve_path(&self, request_path: &str) -> Option<PathBuf> {
        let relative = request_path.trim_start_matches('/');

        let relative = if relative.is_empty() {
            "index.html"
        } else {
            relative
        };

        let path = self.root.join(relative);

        // The code below this comment is to prevent path traversal attacks

        let canonical_root = self.root.canonicalize().ok()?;
        let canonical_path = path.canonicalize().ok()?;

        if canonical_path.starts_with(&canonical_root) {
            Some(canonical_path)
        } else {
            None
        }
    }
}

fn infer_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        // text
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml",
        Some("csv") => "text/csv",

        // images
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",

        // fonts
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",

        // audio / video
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",

        // fallback — tell the browser to download it :P
        _ => "application/octet-stream",
    }
}

struct ReadFileFuture {
    path: PathBuf,
    result: Option<io::Result<Vec<u8>>>,
    rx: Option<std::sync::mpsc::Receiver<io::Result<Vec<u8>>>>,
}

impl ReadFileFuture {
    fn new(path: PathBuf) -> Self {
        ReadFileFuture {
            path,
            result: None,
            rx: None,
        }
    }
}

impl Future for ReadFileFuture {
    type Output = io::Result<Vec<u8>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.rx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            let path = self.path.clone();
            let waker = cx.waker().clone();

            std::thread::spawn(move || {
                let result = std::fs::read(&path);
                let _ = tx.send(result);

                waker.wake();
            });

            self.rx = Some(rx);
            return Poll::Pending;
        }

        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(result) => Poll::Ready(result),
                Err(_) => {
                    cx.waker().clone();
                    Poll::Pending
                }
            }
        } else {
            Poll::Pending
        }
    }
}
