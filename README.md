# minicdn

Static files, compressed for efficiency. Currently, requires Rust nightly.

## Example

In this example, we use a macro to evaluate a path at compile time, relative to the source file.
In debug mode, the files will be loaded at runtime. In release mode, the files are embedded (and
appropriately compressed) into the compiled binary.

```rust
let files: MiniCdn = release_include_mini_cdn!("./path/to/public/files/");

let html = files.get("index.html").unwrap();

// 32 byte digest of the file.
let _ = html.etag;
// Last modified time as string, in UNIX seconds.
let _ = html.last_modified;
// MIME type string.
let _ = html.mime;
// Raw HTML bytes.
let _ = html.contents;
// HTML compressed with Brotli, if it is more efficient.
let _ = html.contents_brotli;
// HTML compressed with GZIP, if it is more efficient.
let _ = html.contents_gzip;

let image = files.get("images/foo.png").unwrap();

// Raw PNG bytes.
let _ = image.contents;

// WebP bytes (if WebP is more efficient).
let _ = image.contents_webp;
```

Check the documentation for other options, such as doing the compression at runtime.

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.