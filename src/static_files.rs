use crate::http::response::HttpResponse;
use std::path::{Path, PathBuf};

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

        let bytes = tokio::fs::read(&file_path).await.ok()?;

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
