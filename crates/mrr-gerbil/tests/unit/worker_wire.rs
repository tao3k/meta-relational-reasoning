use crate::worker_wire::{FRAME_LIMIT, PAYLOAD_LIMIT, decode, encode, read_frame};
#[test]
fn inert_unicode_and_escapes_roundtrip_without_evaluation() {
    let payload = "(object \"因果\" \"a\\b\")\n\t";
    let frame = encode("request", 7, 4, payload);
    assert_eq!(
        decode("request", frame.as_bytes()),
        Some((7, 4, payload.as_bytes().to_vec()))
    );
    assert!(decode("response", frame.as_bytes()).is_none());
    for malformed in [
        "(mrr.temporal-worker.request.v1 0 4 \"\")\n",
        "(mrr.temporal-worker.request.v1 1 4 \"\\x\")\n",
        "(mrr.temporal-worker.request.v1 1 4 \"\") trailing\n",
    ] {
        assert!(decode("request", malformed.as_bytes()).is_none());
    }
}
#[test]
fn oversized_and_truncated_frames_are_rejected() {
    let frame = encode("request", 1, 0, &"x".repeat(PAYLOAD_LIMIT + 1));
    assert!(decode("request", frame.as_bytes()).is_none());
    assert!(read_frame(&mut std::io::Cursor::new(vec![b'x'; FRAME_LIMIT + 1])).is_err());
    assert!(read_frame(&mut std::io::Cursor::new(b"unfinished")).is_err());
}
