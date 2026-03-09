#[derive(Debug, Clone)]
pub enum StatusCode {
    // 2XX Success
    Ok,        // 200
    Created,   // 201
    NoContent, // 204

    // 3XX Redirection
    MovedPermanently, // 301
    Found,            // 302
    NotModified,      // 304

    // 4XX Client Errors
    BadRequest,          // 400
    Unauthorized,        // 401
    Forbidden,           // 403
    NotFound,            // 404
    MethodNotAllowed,    // 405
    UnprocessableEntity, // 422

    // 5XX Server Errors
    InternalServerError, // 500
    NotImplemented,      // 501
    BadGateway,          // 502
    ServiceUnavailable,  // 503
}

impl StatusCode {
    pub fn code(&self) -> u16 {
        match self {
            StatusCode::Ok => 200,
            StatusCode::Created => 201,
            StatusCode::NoContent => 204,
            StatusCode::MovedPermanently => 301,
            StatusCode::Found => 302,
            StatusCode::NotModified => 304,
            StatusCode::BadRequest => 400,
            StatusCode::Unauthorized => 401,
            StatusCode::Forbidden => 403,
            StatusCode::NotFound => 404,
            StatusCode::MethodNotAllowed => 405,
            StatusCode::UnprocessableEntity => 422,
            StatusCode::InternalServerError => 500,
            StatusCode::NotImplemented => 501,
            StatusCode::BadGateway => 502,
            StatusCode::ServiceUnavailable => 503,
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::NoContent => "No Content",
            StatusCode::MovedPermanently => "Moved Permanently",
            StatusCode::Found => "Found",
            StatusCode::NotModified => "Not Modified",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::UnprocessableEntity => "Unprocessable Entity",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadGateway => "Bad Gateway",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}
