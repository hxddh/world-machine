const PROCESS_SOURCE: &str = include_str!("../src/lib.rs");

#[test]
fn pack_request_preflight_precedes_request_id_commit_and_dispatch() {
    assert!(
        PROCESS_SOURCE.contains("DEFAULT_MAX_REQUEST_BYTES"),
        "M261 must define a fixed Pack request wire ceiling in the process client"
    );

    let request_start = PROCESS_SOURCE
        .find("fn request(&mut self, request: PackRequest)")
        .expect("ProcessClient::request must remain present");
    let shutdown_start = PROCESS_SOURCE[request_start..]
        .find("fn send_shutdown(&mut self)")
        .map(|offset| request_start + offset)
        .expect("ProcessClient::send_shutdown must follow request");
    let request_body = &PROCESS_SOURCE[request_start..shutdown_start];

    let prepare = request_body
        .find("prepare_request_frame(")
        .expect("request must prepare and bound the encoded frame before dispatch");
    let commit = request_body
        .find("self.next_request_id =")
        .expect("request must commit the next request id only after preflight");
    let dispatch = request_body
        .find("write_all(&frame)")
        .expect("request must dispatch the already-prepared frame in one logical write");

    assert!(
        prepare < commit,
        "request preparation must happen before request-id correlation state is committed"
    );
    assert!(
        commit < dispatch,
        "request-id correlation state must be committed before the prepared frame is dispatched"
    );
    assert!(
        !request_body.contains("write_all(encoded.as_bytes())"),
        "request transport must not bypass the prepared bounded frame"
    );
}

#[test]
fn pack_shutdown_uses_the_same_bounded_frame_preparation_path() {
    let shutdown_start = PROCESS_SOURCE
        .find("fn send_shutdown(&mut self)")
        .expect("ProcessClient::send_shutdown must remain present");
    let terminate_start = PROCESS_SOURCE[shutdown_start..]
        .find("fn terminate(&mut self)")
        .map(|offset| shutdown_start + offset)
        .expect("ProcessClient::terminate must follow send_shutdown");
    let shutdown_body = &PROCESS_SOURCE[shutdown_start..terminate_start];

    assert!(
        shutdown_body.contains("prepare_request_frame("),
        "shutdown must not retain a second unbounded request encoder/writer"
    );
    assert!(
        shutdown_body.contains("write_all(&frame)"),
        "shutdown must dispatch the same prepared payload+LF frame shape"
    );
    assert!(
        !shutdown_body.contains("write_all(encoded.as_bytes())"),
        "shutdown must not bypass the bounded frame path"
    );
}
