//! Le vrai client HTTP, contre un mini serveur local.

use scriibe_engines::http::{send, Http, HttpRequest, UreqHttp};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

/// Répond une fois `status` avec `body`, et renvoie la requête reçue (en-têtes et corps).
fn serve_once(status: u16, body: &'static str) -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut received = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = stream.read(&mut chunk).unwrap_or(0);
            received.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&received).into_owned();
            if let Some(split) = text.find("\r\n\r\n") {
                let length = text[..split]
                    .lines()
                    .find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                    .unwrap_or(0);
                if received.len() >= split + 4 + length {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        let reason = if status == 200 { "OK" } else { "Error" };
        let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let _ = stream.write_all(response.as_bytes());
        let _ = sender.send(String::from_utf8_lossy(&received).into_owned());
    });
    (port, receiver)
}

#[test]
fn posts_a_body_with_headers_and_reads_the_answer() {
    let (port, received) = serve_once(200, "{\"ok\":true}");
    let request = HttpRequest::post(format!("http://127.0.0.1:{port}/v1/chat/completions"), b"{\"a\":1}".to_vec(), Duration::from_secs(5))
        .header("Authorization", "Bearer secret")
        .header("Content-Type", "application/json");
    let response = UreqHttp::default().send(request).unwrap();
    assert_eq!((response.status, response.body.as_slice()), (200, &b"{\"ok\":true}"[..]));
    let text = received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(text.starts_with("POST /v1/chat/completions HTTP/1.1"), "{text}");
    assert!(text.to_lowercase().contains("authorization: bearer secret"));
    assert!(text.ends_with("{\"a\":1}"));
}

#[test]
fn an_http_error_status_is_a_response_not_a_network_error() {
    let (port, _received) = serve_once(401, "no");
    let request = HttpRequest::get(format!("http://127.0.0.1:{port}/"), Duration::from_secs(5));
    let response = UreqHttp::default().send(request).unwrap();
    assert_eq!((response.status, response.body.as_slice()), (401, &b"no"[..]));

    let (port, _received) = serve_once(401, "no");
    let error =
        send(&UreqHttp::default(), HttpRequest::get(format!("http://127.0.0.1:{port}/"), Duration::from_secs(5)), "Deepgram", false)
            .unwrap_err();
    assert_eq!((error.message.as_str(), error.status), ("Clé Deepgram refusée", Some(401)));
}

#[test]
fn a_closed_port_is_a_network_error() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    assert!(UreqHttp::default().send(HttpRequest::get(format!("http://127.0.0.1:{port}/"), Duration::from_secs(2))).is_err());
}

#[test]
fn a_silent_server_hits_the_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let start = std::time::Instant::now();
    let result = UreqHttp::default().send(HttpRequest::get(format!("http://127.0.0.1:{port}/"), Duration::from_millis(300)));
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(3));
    drop(listener);
}
