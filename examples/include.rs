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
    let mut total_size = 0;
    mini_cdn.for_each(|path, file| {
        println!("{:?}: {:?}", path, file);
        total_size += file.contents.len();
        #[cfg(feature = "brotli")]
        {
            total_size += file
                .contents_brotli
                .as_ref()
                .map(|c| c.len())
                .unwrap_or_default();
        }
        #[cfg(feature = "gzip")]
        {
            total_size += file
                .contents_gzip
                .as_ref()
                .map(|c| c.len())
                .unwrap_or_default();
        }
        #[cfg(feature = "webp")]
        {
            total_size += file
                .contents_webp
                .as_ref()
                .map(|c| c.len())
                .unwrap_or_default();
        }
    });

    #[cfg(feature = "serde")]
    {
        let str = serde_json::to_string(&mini_cdn).unwrap();
        println!("{}", str);
    }

    println!("total_size: {total_size}");
}
