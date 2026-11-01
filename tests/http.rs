use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
#[test]
fn real_http_get_and_post_to_local_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        for count in 0..2 {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0; 1];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() < 16384);
            }
            let header = String::from_utf8(request).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).unwrap();
            if count == 0 {
                assert!(header.starts_with("GET "));
            } else {
                assert!(header.starts_with("POST "));
                assert_eq!(body, b"hello");
            }
            let body = if count == 0 {
                "get worked"
            } else {
                "post worked"
            };
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let source=format!("import net\na=net.get(\"http://127.0.0.1:{port}/\")\nassert(a is success, text(a))\nprint(a.value.status, a.value.body)\nb=net.post(\"http://127.0.0.1:{port}/\",\"hello\")\nassert(b is success, text(b))\nprint(b.value.status, b.value.body)");
    assert_eq!(
        indium::run_source(&source).unwrap(),
        "200 get worked\n200 post worked\n"
    );
    server.join().unwrap();
}
