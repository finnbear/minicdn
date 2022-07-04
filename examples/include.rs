use minicdn::{include_mini_cdn, release_include_mini_cdn, MiniCdn};

pub fn main() {
    println!("Filesystem:");
    dump_mini_cdn(MiniCdn::new_filesystem_from_path("examples/tree".into()));

    println!("Included:");
    dump_mini_cdn(MiniCdn::Embedded(include_mini_cdn!("./tree")));

    println!("Conditional:");
    dump_mini_cdn(release_include_mini_cdn!("./tree"));
}

fn dump_mini_cdn(mini_cdn: MiniCdn) {
    mini_cdn.for_each(|path, file| {
        println!("{:?}: mime={:?} etag={:?} last_modified={:?} contents={:?} brotli={:?} gzip={:?} webp={:?}",
                 path,
                 file.mime.clone(),
                 file.etag.clone(),
                 file.last_modified.clone(),
                 file.contents.len(),
                 file.contents_brotli.as_ref().map(|b| b.len()),
                 file.contents_gzip.as_ref().map(|b| b.len()),
                 file.contents_webp.as_ref().map(|b| b.len())
        );
    });

    #[cfg(feature = "serde")]
    {
        let str = serde_json::to_string(&mini_cdn).unwrap();
        println!("{}", str);
    }
}
