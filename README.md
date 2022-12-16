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

All of the fields (excepts `contents`) are disabled by default, but can be switched on by a corresponding feature flag.

Check the documentation for other options, such as doing the compression at runtime.

## Config file

There is experimental support for customizing compression using a config file. If you had an image named `some_image.png`,
you could place the following in a new file named `some_image.minicdn` to adjust the WebP quality level.

```toml
webp_quality = 75.0
```

The following options are available:
- `brotli_level` (1-11, default 9)
- `brotli_buffer_size` (bytes, default 4096)
- `brotli_large_window_size` (default 20)
- `gzip_level` (1-9, default 9)
- `webp_quality` (0-100 or "lossless", default 90)

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