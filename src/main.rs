use http::{request::HttpRequest, response::HttpResponse, status::StatusCode};
use router::Router;
use runtime::{
    SpawnHandle,
    executor::Executor,
    tcp::{TcpListener, TcpStream},
};
use static_files::StaticFiles;
use std::{sync::Arc, time::Duration};

mod http;
mod router;
mod runtime;
mod static_files;

const READ_TIMEOUT_IN_SECS: u64 = 5;
const MAX_REQUEST_SIZE: usize = 8 * 1024; // 8KB should be enough for any reasonable http request 
const MAX_CONNECTIONS: usize = 100;

const HOST: &'static str = "127.0.0.1";
const PORT: &'static str = "8080";

fn about_handler(_req: HttpRequest) -> HttpResponse {
    HttpResponse::ok(b"<h1>About page</h1>".to_vec(), "text/html")
}

// TODO: check for vulnerabilties over this function
fn echo_handler(req: HttpRequest) -> HttpResponse {
    let body = req.body.unwrap_or_else(|| b"(no body)".to_vec());
    HttpResponse::ok(body, "text/plain")
}

fn slow_handler(_req: HttpRequest) -> HttpResponse {
    std::thread::sleep(Duration::from_secs(5));
    HttpResponse::ok(b"slow as hell".to_vec(), "text/plain")
}

async fn handle_connection(
    mut stream: TcpStream,
    router: Arc<Router>,
    static_files: Arc<StaticFiles>,
) {
    let mut buffer = vec![0u8; MAX_REQUEST_SIZE];

    // Use of a timeout to protect from Slowloris attacks
    let bytes_read = match stream.read(&mut buffer).await {
        Ok(0) => {
            return; // connection closed before sending anything
        }
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read from stream: {}", e);
            return;
        }
    };

    // reject any request that's too large
    if bytes_read == MAX_REQUEST_SIZE {
        let response = HttpResponse::new(StatusCode::BadRequest)
            .header("Content-Type", "text/plain")
            .body(b"400 Request Too Large".to_vec());
        let _ = stream.write_all(&response.to_bytes()).await;
        return;
    }

    let raw = String::from_utf8_lossy(&buffer[..bytes_read]);

    let response = match HttpRequest::parse(&raw) {
        Err(e) => {
            eprintln!("Parse error: {}", e);
            HttpResponse::internal_server_error()
        }
        Ok(req) => {
            if let Some(static_response) = static_files.serve(&req.path).await {
                static_response
            } else {
                router.handle(req)
            }
        }
    };

    if let Err(e) = stream.write_all(&response.to_bytes()).await {
        eprintln!("Failed to write response: {}", e);
    }
}

async fn server(router: Arc<Router>, static_files: Arc<StaticFiles>, spawner: SpawnHandle) {
    let bind_addr = format!("{}:{}", HOST, PORT);
    let listener = TcpListener::bind(bind_addr).expect("Failed to bind to port");

    println!("Server listening on http://{}:{}/", HOST, PORT);

    let connections = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    loop {
        // waiting for an incoming connection
        match listener.accept().await {
            Err(e) => eprintln!("Failed to accept connection: {}", e),
            Ok(stream) => {
                let count = connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                if count >= MAX_CONNECTIONS {
                    connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("Too many connecions, rejecting");
                    continue;
                }

                let router = router.clone();
                let static_files = static_files.clone();
                let connections = connections.clone();

                spawner.spawn(async move {
                    handle_connection(stream, router, static_files).await;
                    connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                });
            }
        }
    }
}

fn main() {
    let router = Arc::new(
        Router::new()
            .get("/about", about_handler)
            .post("/echo", echo_handler)
            .get("/slow", slow_handler), //testing purposes
    );

    let static_files = Arc::new(StaticFiles::new("./static"));

    let (mut executor, spawner) = Executor::new();

    executor.spawn(server(router, static_files, spawner));
    executor.run();
}
