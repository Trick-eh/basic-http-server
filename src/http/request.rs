use std::collections::HashMap;

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Unknown(String),
}

impl Method {
    fn from_str(s: &str) -> Self {
        match s {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "PATCH" => Method::Patch,
            other => Method::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: Method,  // GET, POST, PUT, DELETE...
    pub path: String,    // "/", "/something", "/index.html"
    pub version: String, // "HTTP/1.1"
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum ParseError {
    EmptyRequest,
    MalformedRequestLine,
    MalformedHeader(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyRequest => write!(f, "request is empty"),
            ParseError::MalformedRequestLine => write!(f, "malformed request line"),
            ParseError::MalformedHeader(h) => write!(f, "malformed header: {}", h),
        }
    }
}

impl HttpRequest {
    pub fn parse(raw: &str) -> Result<Self, ParseError> {
        // split the head from body on the blank line
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));

        let mut lines = head.lines();

        // parse request line
        let request_line = lines.next().ok_or(ParseError::EmptyRequest)?;
        let mut parts = request_line.splitn(3, ' ');

        let method = parts
            .next()
            .ok_or(ParseError::MalformedRequestLine)
            .map(Method::from_str)?;

        let path = parts
            .next()
            .ok_or(ParseError::MalformedRequestLine)?
            .to_string();

        let version = parts
            .next()
            .ok_or(ParseError::MalformedRequestLine)?
            .to_string();

        // parse headers
        let mut headers = HashMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            let (key, value) = line
                .split_once(": ")
                .ok_or_else(|| ParseError::MalformedHeader(line.to_string()))?;

            headers.insert(key.to_string(), value.to_string());
        }

        // parse body (there might not be a body) after the \r\n\r\n

        let body = if body.is_empty() {
            None
        } else {
            Some(body.as_bytes().to_vec())
        };

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }

    pub fn content_length(&self) -> usize {
        self.headers
            .get("Content-Length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    pub fn content_type(&self) -> Option<&str> {
        self.headers.get("Content-Type").map(|v| v.as_str())
    }
}
