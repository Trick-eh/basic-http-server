use super::status::StatusCode;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl HttpResponse {
    pub fn new(status: StatusCode) -> Self {
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.headers
            .insert("Content-Length".to_string(), body.len().to_string());
        self.body = Some(body);
        self
    }

    pub fn ok(body: Vec<u8>, content_type: &str) -> Self {
        HttpResponse::new(StatusCode::Ok)
            .header("Content-Type", content_type)
            .body(body)
    }

    pub fn not_found() -> Self {
        let body = b"404 Not Found".to_vec();
        HttpResponse::new(StatusCode::NotFound)
            .header("Content-Type", "text/plain")
            .body(body)
    }

    pub fn internal_server_error() -> Self {
        let body = b"500 Internal Server Error".to_vec();
        HttpResponse::new(StatusCode::InternalServerError)
            .header("Content-Type", "text/plain")
            .body(body)
    }

    pub fn method_not_allowed() -> Self {
        let body = b"405 Method Not Allowed".to_vec();
        HttpResponse::new(StatusCode::MethodNotAllowed)
            .header("Content-Type", "text/plain")
            .body(body)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = String::new();

        // status line
        response.push_str(&format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.code(),
            self.status.reason(),
        ));

        // headers
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }

        // headers separated from the body
        response.push_str("\r\n");

        // convert the header into bytes
        let mut bytes = response.into_bytes();

        // append body (already in bytes) if present
        if let Some(body) = &self.body {
            bytes.extend_from_slice(body);
        }

        bytes
    }
}
