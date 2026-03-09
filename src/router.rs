use crate::http::request::{HttpRequest, Method};
use crate::http::response::HttpResponse;

pub type Handler = fn(HttpRequest) -> HttpResponse;

struct Route {
    method: Method,
    path: String,
    handler: Handler,
}

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Router { routes: Vec::new() }
    }

    pub fn get(mut self, path: &str, handler: Handler) -> Self {
        self.routes.push(Route {
            method: Method::Get,
            path: path.to_string(),
            handler,
        });
        self
    }

    pub fn post(mut self, path: &str, handler: Handler) -> Self {
        self.routes.push(Route {
            method: Method::Post,
            path: path.to_string(),
            handler,
        });
        self
    }

    pub fn put(mut self, path: &str, handler: Handler) -> Self {
        self.routes.push(Route {
            method: Method::Put,
            path: path.to_string(),
            handler,
        });
        self
    }

    pub fn delete(mut self, path: &str, handler: Handler) -> Self {
        self.routes.push(Route {
            method: Method::Delete,
            path: path.to_string(),
            handler,
        });
        self
    }

    pub fn handle(&self, request: HttpRequest) -> HttpResponse {
        for route in &self.routes {
            if route.path == request.path && methods_match(&route.method, &request.method) {
                return (route.handler)(request);
            }
        }

        // no route matched
        HttpResponse::not_found()
    }
}

fn methods_match(a: &Method, b: &Method) -> bool {
    matches!(
        (a, b),
        (Method::Get, Method::Get)
            | (Method::Post, Method::Post)
            | (Method::Put, Method::Put)
            | (Method::Delete, Method::Delete)
            | (Method::Head, Method::Head)
            | (Method::Options, Method::Options)
            | (Method::Patch, Method::Patch)
    )
}
