#![feature(proc_macro_span)]
#![cfg_attr(feature = "track_path", feature(track_path))]

extern crate core;

use litrs::StringLit;
use minicdn_core::EmbeddedMiniCdn;
use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use std::path::Path;

#[proc_macro]
/// This macro evaluates the path relative to the source file.
///
/// # Release mode
///
/// Compresses and embeds files at compile time (may incur significant compile time overhead).
///
/// # Debug mode
///
/// References files so that they be loaded at runtime.
pub fn release_include_mini_cdn(args: TokenStream) -> TokenStream {
    let arg = parse_arg(args);
    let path = arg_to_path(&arg);

    quote! {
        {
            #[cfg(debug_assertions)]
            {
                minicdn::MiniCdn::new_filesystem_from_path(std::borrow::Cow::Borrowed(#path))
            }

            #[cfg(not(debug_assertions))]
            {
                minicdn::MiniCdn::Embedded(minicdn::include_mini_cdn!(#arg))
            }
        }
    }
    .into()
}

#[proc_macro]
/// Compresses and embeds files at compile time (may incur significant compile time overhead).
///
/// This macro evaluates the path relative to the source file.
pub fn include_mini_cdn(args: TokenStream) -> TokenStream {
    let root_path = arg_to_path(&parse_arg(args));

    let mut files = Vec::<proc_macro2::TokenStream>::new();

    #[cfg(feature = "track_dir")]
    proc_macro::tracked_path::path(&root_path);

    #[allow(unused)]
    EmbeddedMiniCdn::new_compressed(&root_path)
        .iter()
        .for_each(|(path, file)| {
            #[cfg(feature = "track_dir")]
            proc_macro::tracked_path::path(path);

            #[allow(unused_mut)]
            let mut fields = Vec::<proc_macro2::TokenStream>::new();

            #[allow(unused)]
            use std::ops::Deref;

            #[cfg(feature = "etag")]
            {
                let etag = file.etag.deref();
                fields.push(quote! {
                    etag: #etag.into()
                });
            }

            #[cfg(feature = "last_modified")]
            {
                let last_modified = file.last_modified.deref();
                fields.push(quote! {
                    last_modified: #last_modified.into()
                });
            }

            #[cfg(feature = "mime")]
            {
                let mime = file.mime.deref();
                fields.push(quote! {
                    mime: #mime.into()
                });
            }

            #[cfg(feature = "brotli")]
            {
                let contents_brotli = quote_option_bytes(&file.contents_brotli);
                fields.push(quote! {
                    contents_brotli: #contents_brotli
                });
            }

            #[cfg(feature = "gzip")]
            {
                let contents_gzip = quote_option_bytes(&file.contents_gzip);
                fields.push(quote! {
                    contents_gzip: #contents_gzip
                });
            }

            #[cfg(feature = "webp")]
            {
                let contents_webp = quote_option_bytes(&file.contents_webp);
                fields.push(quote! {
                    contents_webp: #contents_webp
                });
            }

            let include_path_raw = Path::new(&root_path).join(&**path);
            let include_path_canonical = include_path_raw.canonicalize().expect(&format!(
                "failed to canonicalize include path {:?}",
                include_path_raw
            ));
            let include_path = include_path_canonical.to_str().expect(&format!(
                "failed to stringify include path: {:?}",
                include_path_canonical
            ));

            // Must use include_str! instead of file.contents so a change triggers a recompilation.
            files.push(
                quote! {
                    ret.insert(std::borrow::Cow::Borrowed(#path), minicdn::MiniCdnFile{
                        contents: minicdn::Base64Bytes::from_static(include_bytes!(#include_path)),
                        #(#fields,)*
                    });
                }
                .into(),
            );
        });

    quote! {
        {
            let mut ret = minicdn::EmbeddedMiniCdn::default();
            #(#files)*
            ret
        }
    }
    .into()
}

fn parse_arg(args: TokenStream) -> String {
    let input = args.into_iter().collect::<Vec<_>>();
    if input.len() != 1 {
        panic!("expected exactly one input token, got {}", input.len());
    }
    let string_lit = match StringLit::try_from(&input[0]) {
        // Error if the token is not a string literal
        Err(e) => panic!("error parsing argument: {:?}", e),
        Ok(lit) => lit,
    };
    string_lit.value().to_string()
}

fn arg_to_path(arg: &str) -> String {
    if Path::new(arg).is_absolute() {
        // Absolute path.
        String::from(arg)
    } else {
        // Relative path.
        let mut root_path = proc_macro::Span::call_site().source_file().path();
        // Get rid of the source file name.
        root_path.pop();
        root_path.push(Path::new(arg));
        // Relative paths cause problems when Rust arbitrarily manipulates the working directory
        // e.g. when workspaces are used.
        let canonical = root_path
            .canonicalize()
            .expect(&format!("failed to canonicalize path {:?}", root_path));
        canonical
            .to_str()
            .expect(&format!(
                "failed to stringify canonical path {:?} (root path is {:?})",
                canonical, root_path
            ))
            .to_string()
    }
}

#[derive(Debug)]
struct ByteStr<'a>(pub &'a [u8]);

impl<'a> ToTokens for ByteStr<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(TokenTree::Literal(Literal::byte_string(self.0)));
    }
}

#[allow(unused)]
fn quote_bytes(data: &minicdn_core::Base64Bytes) -> proc_macro2::TokenStream {
    let bytes = ByteStr(data.as_ref());
    quote! {
        minicdn::Base64Bytes::from_static(#bytes)
    }
    .into()
}

#[allow(unused)]
fn quote_option_bytes(opt: &Option<minicdn_core::Base64Bytes>) -> proc_macro2::TokenStream {
    if let Some(data) = opt {
        let bytes = quote_bytes(data);
        quote! {
            Some(#bytes)
        }
    } else {
        quote! {
            None
        }
        .into()
    }
}
