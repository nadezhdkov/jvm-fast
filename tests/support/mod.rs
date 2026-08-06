use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

pub struct MockHttpServer {
    pub base_url: String,
}

/// Servidor HTTP mock mínimo (só entende GET) — existe para os testes de
/// `download`/`pom::http` nunca dependerem de rede real, per
/// docs/CONVENTIONS.md ("testes... usam... servidor HTTP mock... nunca
/// dependem de Maven Central real"). Roda em background pela duração do
/// processo de teste; não precisa de shutdown explícito.
pub fn start_mock_server<F>(respond: F) -> MockHttpServer
where
    F: Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind an ephemeral port");
    let addr = listener.local_addr().expect("should have a local addr");
    let respond = Arc::new(respond);

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let respond = Arc::clone(&respond);
            thread::spawn(move || handle_connection(stream, respond.as_ref()));
        }
    });

    MockHttpServer {
        base_url: format!("http://{addr}"),
    }
}

fn handle_connection(mut stream: std::net::TcpStream, respond: &dyn Fn(&str) -> (u16, Vec<u8>)) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let (status, body) = respond(&path);
    let status_text = match status {
        200 => "200 OK",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {status_text}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );

    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
}
