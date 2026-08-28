const PROCESS_SOURCE: &str = include_str!("../src/lib.rs");

#[test]
fn pack_response_reader_uses_a_fixed_bounded_queue() {
    assert!(
        PROCESS_SOURCE.contains("const RESPONSE_QUEUE_CAPACITY: usize = 1;"),
        "M262 must pin the production Pack response queue to one waiting record"
    );

    let reader_start = PROCESS_SOURCE
        .find("fn spawn_response_reader(")
        .expect("Pack response reader must remain present");
    let line_reader_start = PROCESS_SOURCE[reader_start..]
        .find("fn read_bounded_line(")
        .map(|offset| reader_start + offset)
        .expect("bounded physical-line reader must follow the response reader");
    let response_reader = &PROCESS_SOURCE[reader_start..line_reader_start];

    assert!(
        response_reader.contains("mpsc::sync_channel(RESPONSE_QUEUE_CAPACITY)"),
        "Pack response records must cross a fixed-capacity synchronous queue"
    );
    assert!(
        !response_reader.contains("mpsc::channel()"),
        "the Pack response reader must not retain an unbounded mpsc queue"
    );
}
