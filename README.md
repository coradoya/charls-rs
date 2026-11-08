# charls-rs

[![charls on crates.io](https://img.shields.io/crates/v/charls.svg)](https://crates.io/crates/charls)
[![dependency status](https://deps.rs/repo/github/coradoya/charls-rs/status.svg)](https://deps.rs/repo/github/coradoya/charls-rs)
[![charls documentation](https://docs.rs/charls/badge.svg)](https://docs.rs/charls)

Rust bindings for [CharLS](https://github.com/team-charls/charls),
the implementation of the JPEG-LS standard
for lossless and near-lossless image compression and decompression.

The crate wraps the CharLS C API in a small safe interface: `CharLS::encode`, `CharLS::decode` and
`CharLS::get_frame_info`. It also builds for `wasm32`, where it decodes through the official
CharLS WebAssembly package.

## Install

```toml
[dependencies]
charls = "0.4"
```

By default, `charls-sys` compiles the bundled CharLS sources (CharLS 2.4.5) with CMake when you build.
You need:

- CMake
- A C++ compiler

You do not need to install CharLS on the system.

## Usage

### Decode a JPEG-LS file

```rust,no_run
use charls::CharLS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("image.jls")?;

    let info = CharLS::default().get_frame_info(&data)?;
    let pixels = CharLS::default().decode(&data)?;

    println!(
        "{}x{}, {} bits, {} component(s), {} bytes",
        info.width, info.height, info.bits_per_sample, info.component_count, pixels.len()
    );
    Ok(())
}
```

The decoder writes samples tightly packed. A sample is 1 byte if `bits_per_sample` is 8 or less.
Otherwise, a sample is 2 little-endian bytes.

To decode into rows with padding, use `decode_with_stride(src, stride)`. `stride` is the number of
bytes in one output row. `0` means tightly packed.

### Encode

```rust
use charls::{CharLS, FrameInfo};

fn main() -> Result<(), charls::Error> {
    // 4x4 8-bit grayscale
    let pixels: Vec<u8> = (0..16).map(|i| i * 16).collect();
    let frame = FrameInfo { width: 4, height: 4, bits_per_sample: 8, component_count: 1 };

    // near = 0: lossless
    let lossless = CharLS::default().encode(frame.clone(), 0, &pixels)?;
    assert_eq!(CharLS::default().decode(&lossless)?, pixels);

    // near = 2: each sample may differ by at most 2
    let near_lossless = CharLS::default().encode(frame, 2, &pixels)?;
    println!("{} vs {} bytes", lossless.len(), near_lossless.len());
    Ok(())
}
```

For samples wider than 8 bits, give the input as 2 little-endian bytes per sample, for example
`samples.iter().flat_map(|s| s.to_le_bytes())`.

### Color images

For more than one component, set the interleave mode before you encode. The mode must match the
layout of the input buffer:

| `InterleaveMode` | Input layout     | Encoded scans          |
|------------------|------------------|------------------------|
| `None` (default) | `RRRRGGGGBBBB`   | one scan per component |
| `Line`           | `RGBRGBRGBRGB`   | one scan, by line      |
| `Sample`         | `RGBRGBRGBRGB`   | one scan, by pixel     |

```rust
use charls::{CharLS, FrameInfo, InterleaveMode};

fn main() -> Result<(), charls::Error> {
    // 4x4 RGB, pixel by pixel: RGBRGB...
    let rgb = vec![0u8; 4 * 4 * 3];
    let frame = FrameInfo { width: 4, height: 4, bits_per_sample: 8, component_count: 3 };

    let mut encoder = CharLS::default();
    encoder.set_interleave_mode(InterleaveMode::Sample)?;
    let encoded = encoder.encode(frame, 0, &rgb)?;
    assert_eq!(CharLS::default().decode(&encoded)?, rgb);
    Ok(())
}
```

### One operation per `CharLS` value

On the native backend, a `CharLS` value can do one decode (or one `get_frame_info`) and one encode.
A second call of the same kind returns `Error::JpegLsError { code: 7 }`
("Method call is invalid for the current state"). Create a new `CharLS::default()` for each image.
This also applies to `get_frame_info` followed by `decode`.

### Errors

All operations return `charls::CharlsResult<T>`. CharLS failures are reported as
`Error::JpegLsError { code }`. Their `Display` text is the CharLS error message, for example:

```text
Invalid JPEG-LS stream: the leading start byte (0xFF) for a JPEG marker was not found
```

## Cargo features

| Feature  | Effect |
|----------|--------|
| `static` | Link the CharLS library that CMake builds as a static library. |
| `vcpkg`  | Do not build CharLS. Find an installed CharLS with [vcpkg](https://vcpkg.io) instead (`vcpkg install charls`). |

## WebAssembly (`wasm32`)

On `wasm32` targets, the crate does not use `charls-sys`. It calls
[`@team-charls/charls-wasm`](https://www.npmjs.com/package/@team-charls/charls-wasm) through
`wasm-bindgen`. Your code uses the same API on both targets, with these differences:

- Load the module once with `charls::init().await?` before the first decode. Without it, a decode
  returns `Error::Uninitialised`. After `init`, decoding is synchronous.
- Only decoding is available. `encode` and `set_interleave_mode` return `Error::Unsupported`.
- `decode_with_stride` accepts only `stride == 0`.
- Errors do not carry a CharLS error code.

The JavaScript side imports `@team-charls/charls-wasm`, so:

- Build with `wasm-bindgen --target bundler` (or `wasm-pack build --target bundler`).
- Add the npm package to your web application:

  ```sh
  npm install @team-charls/charls-wasm
  ```

```rust,ignore
async fn decode(data: &[u8]) -> charls::CharlsResult<Vec<u8>> {
    charls::init().await?; // loads the module only on the first call
    charls::CharLS::default().decode(data)
}
```

## License

MIT. See [LICENSE.md](LICENSE.md). CharLS is distributed under the BSD-3-Clause license.
