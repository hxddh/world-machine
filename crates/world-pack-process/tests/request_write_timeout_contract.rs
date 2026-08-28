use std::fs;
use std::path::PathBuf;

fn process_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    fs::read_to_string(path).expect("world-pack-process source must be readable")
}

fn request_body(source: &str) -> &str {
    let start = source
        .find("    fn request(&mut self, request: PackRequest) -> Result<PackResponse, HostError> {")
        .expect("ProcessClient::request must exist");
    let tail = &source[start..];
    let end = tail
        .find("\n    fn send_shutdown(&mut self) {")
        .expect("ProcessClient::send_shutdown must follow request");
    &tail[..end]
}

#[test]
fn pack_request_timeout_covers_dispatch_before_response_wait() {
    let source = process_source();
    let body = request_body(&source);

    assert!(
        !body.contains("stdin.write_all(&frame).and_then(|_| stdin.flush())"),
        "ProcessClient::request still performs an unbounded synchronous ChildStdin write before any timeout can fire"
    );

    let timeout = body
        .find("self.request_timeout")
        .expect("request timeout must participate in request dispatch");
    let response_wait = body
        .find("self.responses.recv_timeout")
        .expect("response wait must remain timeout-bounded");
    assert!(
        timeout < response_wait,
        "request timeout is only consulted at the response wait; it must already bound request dispatch"
    );
}
