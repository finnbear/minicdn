use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

/// A collection of files, either loaded from the compiled binary or the filesystem at runtime.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MiniCdn {
    Embedded(EmbeddedMiniCdn),
    Filesystem(FilesystemMiniCdn),
}

/// A collection of files loaded from the compiled binary.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmbeddedMiniCdn {
    files: HashMap<Cow<'static, str>, MiniCdnFile>,
}

/// A collection of files loaded from the filesystem at runtime.
#[derive(Clone)]
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
    /// MIME type.
    pub mime: Cow<'static, str>,
    /// For ETAG-based caching.
    pub etag: Cow<'static, str>,
    /// For last modified caching.
    pub last_modified: Cow<'static, str>,
    /// Raw bytes of file.
    pub contents: Cow<'static, [u8]>,
    /// Contents compressed as Brotli.
    pub contents_brotli: Option<Cow<'static, [u8]>>,
    /// Contents compressed as GZIP.
    pub contents_gzip: Option<Cow<'static, [u8]>>,
    /// Contents compressed as WebP (only applies to images).
    pub contents_webp: Option<Cow<'static, [u8]>>,
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
        get_paths(root_path).for_each(|(absolute_path, relative_path)| {
            let last_modified = last_modified(&absolute_path);
            let contents = std::fs::read(&absolute_path).expect(&relative_path);

            let mime_essence = mime(&relative_path);
            let etag = etag(&contents);
            let contents_webp = webp(&contents, &mime_essence);
            let (contents_brotli, contents_gzip) = if contents_webp.is_none() {
                (brotli(&contents), gzip(&contents))
            } else {
                // Don't apply generic compression to images.
                (None, None)
            };

            ret.insert(
                Cow::Owned(relative_path),
                MiniCdnFile {
                    mime: Cow::Owned(mime_essence),
                    etag: Cow::Owned(etag),
                    last_modified: Cow::Owned(last_modified),
                    contents: Cow::Owned(contents),
                    contents_brotli: contents_brotli.map(Cow::Owned),
                    contents_gzip: contents_gzip.map(Cow::Owned),
                    contents_webp: contents_webp.map(Cow::Owned),
                },
            )
        });
        ret
    }

    /// Gets a previously embedded or inserted file.
    pub fn get(&self, path: &str) -> Option<Cow<'_, MiniCdnFile>> {
        self.files.get(path).map(|f| Cow::Borrowed(f))
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
            mime: Cow::Owned(mime(canonical_path)),
            etag: Cow::Owned(etag(&contents)),
            last_modified: Cow::Owned(last_modified(canonical_path)),
            contents: Cow::Owned(contents),
            contents_brotli: None,
            contents_gzip: None,
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
            Self::Embedded(embedded) => embedded.get(path),
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
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(move |e| {
            let relative_path = e
                .path()
                .strip_prefix(&root_path)
                .unwrap()
                .to_str()
                .expect("relative path error");
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

fn mime(path: &str) -> String {
    mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string()
}

fn last_modified(absolute_path: &str) -> String {
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
                .unwrap()
                .as_secs(),
        )
        .to_string()
}

fn etag(contents: &[u8]) -> String {
    let mut etag = sha256::digest_bytes(contents);
    etag.truncate(32);
    etag
}

fn brotli(contents: &[u8]) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut output = Vec::new();
    let mut writer = brotli::CompressorWriter::new(&mut output, 4096, 9, 22);
    writer.write_all(contents).unwrap();
    drop(writer);
    if output.len() * 10 / 9 < contents.len() {
        Some(output)
    } else {
        // Compression is counterproductive.
        None
    }
}

fn gzip(contents: &[u8]) -> Option<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(contents.as_ref()).unwrap();
    let vec = encoder.finish().unwrap();
    if vec.len() * 10 / 9 < contents.as_ref().len() {
        Some(vec)
    } else {
        // Compression is counterproductive.
        None
    }
}

fn webp(contents: &[u8], mime_essence: &str) -> Option<Vec<u8>> {
    use std::io::Cursor;
    let cursor = Cursor::new(contents.as_ref());
    let mut reader = image::io::Reader::new(cursor);
    use image::ImageFormat;
    reader.set_format(match mime_essence {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        _ => return None,
    });
    match reader.decode() {
        Ok(image) => {
            let webp_image =
                webp::Encoder::from_rgba(image.as_bytes(), image.width(), image.height())
                    .encode(90.0);

            if webp_image.len() * 10 / 9 < contents.len() {
                // Compression is counterproductive.
                Some(webp_image.as_ref().to_vec())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
