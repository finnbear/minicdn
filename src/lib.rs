pub use minicdn_core::{EmbeddedMiniCdn, FilesystemMiniCdn, MiniCdn, MiniCdnFile};
pub use minicdn_macros::{include_mini_cdn, release_include_mini_cdn};

#[cfg(test)]
mod tests {
    use minicdn_core::MiniCdn;
    use std::borrow::Cow;

    #[test]
    fn simple() {
        simple_tests(MiniCdn::new_filesystem_from_path(Cow::Borrowed(
            "examples/tree",
        )));
        simple_tests(MiniCdn::new_embedded_from_path("examples/tree"));

        fn simple_tests(cdn: MiniCdn) {
            assert!(cdn.get("index.html").is_some());
            assert!(cdn.get("/index.html").is_none());
            assert!(cdn.get("subtree/some_binary.bin").is_some());
            assert!(cdn.get("../include.rs").is_none());
        }
    }
}