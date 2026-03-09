use http::{request::HttpRequest, response::HttpResponse, status::StatusCode};
use router::Router;
use static_files::StaticFiles;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::{Duration, timeout},
};

mod http;
mod router;
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

async fn handle_connection(
    mut stream: TcpStream,
    router: Arc<Router>,
    static_files: Arc<StaticFiles>,
) {
    let mut buffer = vec![0u8; MAX_REQUEST_SIZE];

    // Use of a timeout to protect from Slowloris attacks
    let bytes_read = match timeout(
        Duration::from_secs(READ_TIMEOUT_IN_SECS),
        stream.read(&mut buffer),
    )
    .await
    {
        Ok(Ok(0)) => {
            return; // connection closed before sending anything
        }
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            eprintln!("Failed to read from stream: {}", e);
            return;
        }
        Err(_) => {
            eprintln!("Read timeout");
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

#[tokio::main]
async fn main() {
    let router = Arc::new(
        Router::new()
            .get("/about", about_handler)
            .post("/echo", echo_handler),
    );

    let static_files = Arc::new(StaticFiles::new("./static"));
    let semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));

    let bind_addr = format!("{}:{}", HOST, PORT);

    // TODO: receive via os::args the port to which listen
    let listener = TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to port");

    println!("Server listening on http://127.0.0.1:8080");

    loop {
        // waiting for an incoming connection
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        println!("New connection from: {}", addr);

        // Don't allow attackers to starve the resource of the server via DoS
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                eprintln!("Too many connecions, rejecting");
                continue;
            }
        };

        let router = router.clone();
        let static_files = static_files.clone();

        tokio::spawn(async move {
            let _permit = permit;
            handle_connection(stream, router, static_files).await;
        });
    }
}
