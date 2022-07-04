#![feature(proc_macro_span)]

use litrs::StringLit;
use minicdn_core::{EmbeddedMiniCdn, MiniCdnFile};
use proc_macro::TokenStream;
use quote::quote;
use std::borrow::Cow;
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
                MiniCdn::Embedded(minicdn::include_mini_cdn!(#arg))
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

    EmbeddedMiniCdn::new_compressed(&root_path).iter().for_each(
        |(
            path,
            MiniCdnFile {
                mime,
                etag,
                last_modified,
                contents,
                contents_brotli,
                contents_gzip,
                contents_webp,
            },
        )| {
            let contents = quote_cow(contents);
            let contents_brotli = quote_option_cow(contents_brotli);
            let contents_gzip = quote_option_cow(contents_gzip);
            let contents_webp = quote_option_cow(contents_webp);

            files.push(
                quote! {
                    ret.insert(std::borrow::Cow::Borrowed(#path), minicdn::MiniCdnFile{
                        mime: std::borrow::Cow::Borrowed(#mime),
                        etag: std::borrow::Cow::Borrowed(#etag),
                        last_modified: std::borrow::Cow::Borrowed(#last_modified),
                        contents: #contents,
                        contents_brotli: #contents_brotli,
                        contents_gzip: #contents_gzip,
                        contents_webp: #contents_webp,
                    });
                }
                .into(),
            );
        },
    );

    quote! {
        {
            let mut ret = minicdn_core::EmbeddedMiniCdn::default();
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
    let mut root_path = proc_macro::Span::call_site().source_file().path();
    root_path.pop();
    root_path.push(Path::new(arg));
    root_path.to_str().unwrap().to_string()
}

fn quote_cow(data: &Cow<'static, [u8]>) -> proc_macro2::TokenStream {
    quote! {
        std::borrow::Cow::Borrowed(&[
            #(#data,)*
        ])
    }
    .into()
}

fn quote_option_cow(opt: &Option<Cow<'static, [u8]>>) -> proc_macro2::TokenStream {
    if let Some(data) = opt {
        quote! {
            Some(std::borrow::Cow::Borrowed(&[
                #(#data,)*
            ]))
        }
    } else {
        quote! {
            None
        }
        .into()
    }
}
