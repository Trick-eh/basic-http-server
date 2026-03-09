use http::{request::HttpRequest, response::HttpResponse};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

mod http;
#[tokio::main]
async fn main() {
    // TODO: receive via os::args the port to which listen
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind to port");

    println!("Server listening on http://127.0.0.1:8080");

    loop {
        // waiting for an incoming connection
        let (mut stream, addr) = listener
            .accept()
            .await
            .expect("Failed to accept connection");

        println!("New connection from: {}", addr);

        // read raw bytes from the stream into a buffer
        let mut buffer = vec![0u8; 1024];
        let bytes_read = stream
            .read(&mut buffer)
            .await
            .expect("Failed to read from stream");

        let raw = String::from_utf8_lossy(&buffer[..bytes_read]);

        let response = match HttpRequest::parse(&raw) {
            Ok(req) => {
                println!("{:#?}", req);
                HttpResponse::ok(
                    b"<h1>Hello from your HTTP server!</h1>".to_vec(),
                    "text/html",
                )
            }
            Err(e) => {
                println!("Parse error: {}", e);
                HttpResponse::internal_server_error()
            }
        };

        stream
            .write_all(&response.to_bytes())
            .await
            .expect("Failed to write response");
    }
}
