#![cfg(not(target_arch = "wasm32"))]

use charls::{CharLS, Error, FrameInfo, InterleaveMode};

fn gray8() -> (FrameInfo, Vec<u8>) {
    let frame = FrameInfo { width: 4, height: 4, bits_per_sample: 8, component_count: 1 };
    (frame, (0..16).map(|i| i * 16).collect())
}

fn rgb8() -> (FrameInfo, Vec<u8>) {
    let frame = FrameInfo { width: 4, height: 4, bits_per_sample: 8, component_count: 3 };
    (frame, (0..48).map(|i| i * 5).collect())
}

#[test]
fn lossless_round_trip() {
    let (frame, pixels) = gray8();
    let encoded = CharLS::default().encode(frame.clone(), 0, &pixels).unwrap();

    assert_eq!(CharLS::default().get_frame_info(&encoded).unwrap(), frame);
    assert_eq!(CharLS::default().decode(&encoded).unwrap(), pixels);
}

#[test]
fn near_lossless_stays_within_near() {
    let (frame, pixels) = gray8();
    let encoded = CharLS::default().encode(frame, 2, &pixels).unwrap();
    let decoded = CharLS::default().decode(&encoded).unwrap();

    assert_eq!(decoded.len(), pixels.len());
    assert!(pixels.iter().zip(&decoded).all(|(a, b)| a.abs_diff(*b) <= 2));
}

#[test]
fn samples_over_8_bits_are_two_little_endian_bytes() {
    let frame = FrameInfo { width: 4, height: 4, bits_per_sample: 12, component_count: 1 };
    let bytes: Vec<u8> = (0..16u16).flat_map(|i| (i * 250).to_le_bytes()).collect();
    let encoded = CharLS::default().encode(frame, 0, &bytes).unwrap();

    assert_eq!(CharLS::default().decode(&encoded).unwrap(), bytes);
}

#[test]
fn one_instance_decodes_many_times() {
    let (frame, pixels) = gray8();
    let encoded = CharLS::default().encode(frame.clone(), 0, &pixels).unwrap();

    let mut codec = CharLS::default();
    assert_eq!(codec.get_frame_info(&encoded).unwrap(), frame);
    assert_eq!(codec.decode(&encoded).unwrap(), pixels);
    assert_eq!(codec.decode(&encoded).unwrap(), pixels);
}

#[test]
fn one_instance_encodes_many_times() {
    let (frame, pixels) = gray8();
    let mut codec = CharLS::default();
    let first = codec.encode(frame.clone(), 0, &pixels).unwrap();
    let second = codec.encode(frame, 0, &pixels).unwrap();

    assert_eq!(first, second);
    assert_eq!(codec.decode(&second).unwrap(), pixels);
}

#[test]
fn interleave_mode_applies_to_every_encode() {
    let (frame, rgb) = rgb8();
    let mut planar = CharLS::default();
    let none = planar.encode(frame.clone(), 0, &rgb).unwrap();

    let mut codec = CharLS::default();
    codec.set_interleave_mode(InterleaveMode::Sample).unwrap();
    let first = codec.encode(frame.clone(), 0, &rgb).unwrap();
    let second = codec.encode(frame, 0, &rgb).unwrap();

    assert_ne!(first, none);
    assert_eq!(first, second);
    assert_eq!(codec.decode(&second).unwrap(), rgb);
}

#[test]
fn invalid_stream_reports_the_charls_error() {
    let mut codec = CharLS::default();
    let err = codec.decode(&[1, 2, 3]).unwrap_err();

    assert!(matches!(err, Error::JpegLsError { .. }));
    assert!(err.to_string().contains("0xFF"), "{err}");

    // A failed decode does not poison the instance.
    let (frame, pixels) = gray8();
    let encoded = codec.encode(frame, 0, &pixels).unwrap();
    assert_eq!(codec.decode(&encoded).unwrap(), pixels);
}

#[test]
fn codec_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CharLS>();
}
