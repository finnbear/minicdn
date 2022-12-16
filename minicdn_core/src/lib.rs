mod bytes;

pub use crate::bytes::Base64Bytes;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use walkdir::DirEntry;

/// File names with this suffix will be treated as config files
#[cfg(feature = "config")]
pub const CONFIG_SUFFIX: &str = ".minicdn";

/// A collection of files, either loaded from the compiled binary or the filesystem at runtime.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MiniCdn {
    Embedded(EmbeddedMiniCdn),
    Filesystem(FilesystemMiniCdn),
}

/// A collection of files loaded from the compiled binary.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmbeddedMiniCdn {
    files: HashMap<Cow<'static, str>, MiniCdnFile>,
}

/// A collection of files loaded from the filesystem at runtime.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FilesystemMiniCdn {
    root_path: Cow<'static, str>,
}

impl Default for MiniCdn {
    fn default() -> Self {
        Self::Embedded(EmbeddedMiniCdn::default())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MiniCdnFile {
    /// For ETAG-based caching.
    #[cfg(feature = "etag")]
    pub etag: bytestring::ByteString,
    /// For last modified caching.
    #[cfg(feature = "last_modified")]
    pub last_modified: bytestring::ByteString,
    /// MIME type.
    #[cfg(feature = "mime")]
    pub mime: bytestring::ByteString,
    /// Raw bytes of file.
    pub contents: Base64Bytes,
    /// Contents compressed as Brotli.
    #[cfg(feature = "brotli")]
    pub contents_brotli: Option<Base64Bytes>,
    /// Contents compressed as GZIP.
    #[cfg(feature = "gzip")]
    pub contents_gzip: Option<Base64Bytes>,
    /// Contents compressed as WebP (only applies to images).
    #[cfg(feature = "webp")]
    pub contents_webp: Option<Base64Bytes>,
}

impl EmbeddedMiniCdn {
    /// Embeds the files into the binary at runtime, without compressing. The path is evaluated
    /// at runtime.
    pub fn new(root_path: &str) -> Self {
        FilesystemMiniCdn::new(Cow::Owned(root_path.to_string()))
            .borrow()
            .into()
    }

    /// Embeds the files into the binary at runtime. The path and compression are evaluated at
    /// runtime. This may incur significant runtime latency.
    pub fn new_compressed(root_path: &str) -> Self {
        let mut ret = Self::default();

        #[cfg(feature = "brotli")]
        fn default_brotli_level() -> u8 {
            9
        }

        #[cfg(feature = "brotli")]
        fn default_brotli_buffer_size() -> usize {
            4096
        }

        #[cfg(feature = "brotli")]
        fn default_brotli_large_window_size() -> u8 {
            20
        }

        #[cfg(feature = "gzip")]
        fn default_gzip_level() -> u8 {
            8
        }

        #[cfg(feature = "webp")]
        fn default_webp_quality() -> Option<f32> {
            Some(90.0)
        }

        #[cfg_attr(feature = "config", derive(serde::Deserialize))]
        struct Config {
            #[cfg(feature = "brotli")]
            #[cfg_attr(feature = "config", serde(default = "default_brotli_level"))]
            brotli_level: u8,
            #[cfg(feature = "brotli")]
            #[cfg_attr(feature = "config", serde(default = "default_brotli_buffer_size"))]
            brotli_buffer_size: usize,
            #[cfg(feature = "brotli")]
            #[cfg_attr(
                feature = "config",
                serde(default = "default_brotli_large_window_size")
            )]
            brotli_large_window_size: u8,
            #[cfg(feature = "gzip")]
            #[cfg_attr(feature = "config", serde(default = "default_gzip_level"))]
            gzip_level: u8,
            #[cfg(feature = "webp")]
            #[cfg_attr(
                feature = "config",
                serde(
                    default = "default_webp_quality",
                    deserialize_with = "deserialize_webp_quality"
                )
            )]
            webp_quality: Option<f32>,
        }

        #[cfg(all(feature = "webp", feature = "config"))]
        fn deserialize_webp_quality<'de, D: serde::de::Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<f32>, D::Error> {
            struct QualityOrLossless;

            impl<'de> serde::de::Visitor<'de> for QualityOrLossless {
                type Value = Option<f32>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("f32 quality or string \"lossless\"")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if value == "lossless" {
                        Ok(None)
                    } else {
                        Err(E::invalid_value(
                            serde::de::Unexpected::Str(value),
                            &"the string \"lossless\"",
                        ))
                    }
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if (0f64..=100f64).contains(&v) {
                        Ok(Some(v as f32))
                    } else {
                        Err(E::invalid_value(
                            serde::de::Unexpected::Float(v),
                            &"a quality between 0 and 100",
                        ))
                    }
                }
            }

            deserializer.deserialize_any(QualityOrLossless)
        }

        impl Default for Config {
            fn default() -> Self {
                Self {
                    #[cfg(feature = "brotli")]
                    brotli_level: default_brotli_level(),
                    #[cfg(feature = "brotli")]
                    brotli_buffer_size: default_brotli_buffer_size(),
                    #[cfg(feature = "brotli")]
                    brotli_large_window_size: default_brotli_large_window_size(),
                    #[cfg(feature = "gzip")]
                    gzip_level: default_gzip_level(),
                    #[cfg(feature = "webp")]
                    webp_quality: default_webp_quality(),
                }
            }
        }

        #[cfg(feature = "config")]
        let mut configs = HashMap::<String, Config>::new();

        get_paths(root_path).for_each(|(absolute_path, relative_path)| {
            let contents = std::fs::read(&absolute_path).expect(&relative_path);

            #[cfg(feature = "config")]
            if let Some(name) = relative_path.strip_suffix(CONFIG_SUFFIX) {
                let config: Config = toml::from_slice(&contents).expect(&relative_path);
                configs.insert(name.to_owned(), config);
                return;
            }

            #[cfg(feature = "last_modified")]
            let last_modified = last_modified(&absolute_path);
            #[cfg(any(feature = "mime", feature = "webp"))]
            let mime = mime(&relative_path);
            #[cfg(feature = "etag")]
            let etag = etag(&contents);

            #[cfg(feature = "config")]
            #[allow(unused)]
            let config = configs
                .remove({
                    if let Some((before, after)) = relative_path.split_once('.') {
                        before
                    } else {
                        &relative_path
                    }
                })
                .unwrap_or_default();
            #[cfg(not(feature = "config"))]
            #[allow(unused)]
            let config = Config::default();

            #[cfg(feature = "webp")]
            let contents_webp = webp(&contents, &mime, config.webp_quality);

            #[cfg(not(feature = "webp"))]
            #[allow(unused)]
            let special = false;

            #[cfg(feature = "webp")]
            #[allow(unused)]
            let special = contents_webp.is_some();

            #[cfg(feature = "gzip")]
            let contents_gzip = if special {
                None
            } else {
                gzip(&contents, config.gzip_level)
            };

            #[cfg(feature = "brotli")]
            let contents_brotli = if special {
                None
            } else {
                brotli(
                    &contents,
                    config.brotli_buffer_size,
                    config.brotli_level,
                    config.brotli_large_window_size,
                )
            };

            ret.insert(
                Cow::Owned(relative_path),
                MiniCdnFile {
                    #[cfg(feature = "etag")]
                    etag: etag.into(),
                    #[cfg(feature = "last_modified")]
                    last_modified: last_modified.into(),
                    #[cfg(feature = "mime")]
                    mime: mime.into(),
                    contents: contents.into(),
                    #[cfg(feature = "brotli")]
                    contents_brotli: contents_brotli.map(Into::into),
                    #[cfg(feature = "gzip")]
                    contents_gzip: contents_gzip.map(Into::into),
                    #[cfg(feature = "webp")]
                    contents_webp: contents_webp.map(Into::into),
                },
            );
        });

        #[cfg(feature = "config")]
        assert!(
            configs.is_empty(),
            "unused minicdn config files: {:?}",
            configs.keys().collect::<Vec<_>>()
        );

        ret
    }

    /// Gets a previously embedded or inserted file.
    pub fn get(&self, path: &str) -> Option<&MiniCdnFile> {
        self.files.get(path)
    }

    /// Inserts a file.
    pub fn insert(&mut self, path: Cow<'static, str>, file: MiniCdnFile) {
        self.files.insert(path, file);
    }

    /// Removes a file.
    pub fn remove(&mut self, path: &str) {
        self.files.remove(path);
    }

    /// Iterates the previously embedded or inserted files.
    pub fn iter(&self) -> impl Iterator<Item = (&Cow<'_, str>, &MiniCdnFile)> {
        self.files.iter()
    }
}

impl FilesystemMiniCdn {
    /// References the files. Subsequent accesses will load from the file system relative to
    /// this path.
    pub fn new(root_path: Cow<'static, str>) -> Self {
        Self { root_path }
    }

    /// Loads a file from the corresponding directory.
    pub fn get(&self, path: &str) -> Option<MiniCdnFile> {
        #[cfg(feature = "config")]
        if path.ends_with(CONFIG_SUFFIX) {
            // Though we don't expect to be asked for the config file,
            // make sure we never return it.
            return None;
        }

        let canonical_path_tmp = Path::new(self.root_path.as_ref())
            .join(path)
            .canonicalize()
            .ok()?;
        let canonical_path = canonical_path_tmp.to_str()?;
        let canonical_root_path_tmp = Path::new(self.root_path.as_ref()).canonicalize().ok()?;
        let canonical_root_path = canonical_root_path_tmp.to_str()?;
        if !canonical_path.starts_with(canonical_root_path) {
            return None;
        }
        let contents = std::fs::read(&canonical_path).ok()?;
        Some(MiniCdnFile {
            #[cfg(feature = "mime")]
            mime: mime(canonical_path).into(),
            #[cfg(feature = "etag")]
            etag: etag(&contents).into(),
            #[cfg(feature = "last_modified")]
            last_modified: last_modified(canonical_path).into(),
            contents: contents.into(),
            #[cfg(feature = "brotli")]
            contents_brotli: None,
            #[cfg(feature = "gzip")]
            contents_gzip: None,
            #[cfg(feature = "webp")]
            contents_webp: None,
        })
    }

    /// Iterate files in the corresponding directory, without compressing.
    pub fn iter(&self) -> impl Iterator<Item = (String, MiniCdnFile)> + '_ {
        get_paths(&self.root_path).filter_map(|(_, relative)| {
            let file = self.get(&relative)?;
            Some((relative, file))
        })
    }
}

impl MiniCdn {
    /// Embeds the files into the binary at runtime, without compressing. The path is evaluated
    /// at runtime.
    pub fn new_embedded_from_path(root_path: &str) -> Self {
        Self::Embedded(EmbeddedMiniCdn::new(root_path))
    }

    /// Embeds the files into the binary at runtime. The path and compression are evaluated at
    /// runtime. This may incur significant runtime latency.
    pub fn new_compressed_from_path(root_path: &str) -> Self {
        Self::Embedded(EmbeddedMiniCdn::new_compressed(root_path))
    }

    /// References the files. Subsequent accesses will load from the file system relative to
    /// this path.
    pub fn new_filesystem_from_path(root_path: Cow<'static, str>) -> Self {
        Self::Filesystem(FilesystemMiniCdn::new(root_path))
    }

    /// Get a file by path.
    pub fn get(&self, path: &str) -> Option<Cow<'_, MiniCdnFile>> {
        match self {
            Self::Embedded(embedded) => embedded.get(path).map(Cow::Borrowed),
            Self::Filesystem(filesystem) => filesystem.get(path).map(Cow::Owned),
        }
    }

    /// Insert a new file. Will convert to embedded mode if needed.
    pub fn insert(&mut self, path: Cow<'static, str>, file: MiniCdnFile) {
        match self {
            Self::Embedded(embedded) => embedded.insert(path, file),
            Self::Filesystem(filesystem) => {
                *self = Self::Embedded((&*filesystem).into());
                self.insert(path, file);
            }
        }
    }

    /// Apply a function to each file.
    pub fn for_each(&self, mut f: impl FnMut(&str, &MiniCdnFile)) {
        match self {
            Self::Embedded(embedded) => embedded.iter().for_each(|(path, file)| f(&path, &file)),
            Self::Filesystem(filesystem) => {
                filesystem.iter().for_each(|(path, file)| f(&path, &file))
            }
        }
    }
}

impl From<&FilesystemMiniCdn> for EmbeddedMiniCdn {
    fn from(filesystem: &FilesystemMiniCdn) -> Self {
        let mut ret = EmbeddedMiniCdn::default();
        for (existing_path, existing_file) in filesystem.iter() {
            ret.insert(Cow::Owned(existing_path), existing_file);
        }
        ret
    }
}

fn get_paths(root_path: &str) -> impl Iterator<Item = (String, String)> + '_ {
    walkdir::WalkDir::new(&root_path)
        .follow_links(true)
        .sort_by(|a, b| {
            #[derive(Ord, PartialOrd, Eq, PartialEq)]
            struct Priority<'a> {
                #[cfg(feature = "config")]
                real: bool,
                file_name: &'a OsStr,
            }

            fn prioritize(e: &DirEntry) -> Priority<'_> {
                #[cfg(feature = "config")]
                let mut real = true;

                #[cfg(feature = "config")]
                if let Some(string) = e.file_name().to_str() {
                    if string.ends_with(CONFIG_SUFFIX) {
                        real = false;
                    }
                }

                Priority {
                    #[cfg(feature = "config")]
                    real,
                    file_name: e.file_name(),
                }
            }

            prioritize(a).cmp(&prioritize(b))
        })
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(move |e| {
            let relative_path = e
                .path()
                .strip_prefix(&root_path)
                .expect("failed to strip relative path prefix")
                .to_str()
                .expect("failed to stringify relative path");
            let absolute_path_raw =
                std::fs::canonicalize(e.path()).expect("absolute path raw error");
            let absolute_path = absolute_path_raw.to_str().expect("absolute path error");

            let relative_path = if std::path::MAIN_SEPARATOR == '\\' {
                relative_path.replace('\\', "/")
            } else {
                relative_path.to_string()
            };

            (absolute_path.to_string(), relative_path)
        })
}

#[cfg(any(feature = "mime", feature = "webp"))]
fn mime(path: &str) -> String {
    mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string()
}

#[cfg(feature = "last_modified")]
fn last_modified(absolute_path: &str) -> String {
    use std::time::SystemTime;
    std::fs::metadata(absolute_path)
        .expect(&format!("could not get metadata for {}", absolute_path))
        .modified()
        .ok()
        .map(|last_modified| {
            last_modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("invalid UNIX time")
                .as_secs()
        })
        .unwrap_or(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("unix time overflow")
                .as_secs(),
        )
        .to_string()
}

#[cfg(feature = "etag")]
fn etag(contents: &[u8]) -> String {
    let mut etag = sha256::digest_bytes(contents);
    etag.truncate(32);
    //etag.shrink_to_fit();
    etag
}

#[cfg(feature = "brotli")]
fn brotli(contents: &[u8], buffer_size: usize, quality: u8, lgwin: u8) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut output = Vec::new();
    let mut writer =
        brotli::CompressorWriter::new(&mut output, buffer_size, quality as u32, lgwin as u32);
    writer.write_all(contents).unwrap();
    drop(writer);
    if output.len() * 10 / 9 < contents.len() {
        Some(output)
    } else {
        // Compression is counterproductive.
        None
    }
}

#[cfg(feature = "gzip")]
fn gzip(contents: &[u8], level: u8) -> Option<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level as u32));
    encoder.write_all(contents.as_ref()).unwrap();
    let vec = encoder.finish().unwrap();
    if vec.len() * 10 / 9 < contents.len() {
        Some(vec)
    } else {
        // Compression is counterproductive.
        None
    }
}

#[cfg(feature = "webp")]
fn webp(contents: &[u8], mime_essence: &str, quality: Option<f32>) -> Option<Vec<u8>> {
    use std::io::Cursor;
    let cursor = Cursor::new(contents);
    let mut reader = image::io::Reader::new(cursor);
    use image::ImageFormat;
    reader.set_format(match mime_essence {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        _ => return None,
    });
    match reader.decode() {
        Ok(image) => {
            let encoder = webp::Encoder::from_rgba(image.as_bytes(), image.width(), image.height());

            let webp_image = if let Some(quality) = quality {
                encoder.encode(quality)
            } else {
                encoder.encode_lossless()
            };

            if webp_image.len() * 10 / 9 < contents.len() {
                // Compression is counterproductive.
                use std::ops::Deref;
                Some(webp_image.deref().to_vec())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
