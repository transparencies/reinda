use std::collections::HashMap;

use proc_macro::{TokenStream as TokenStream1};
use proc_macro2::{Literal, TokenStream};
use quote::quote;

mod parse;


#[proc_macro]
pub fn assets(input: TokenStream1) -> TokenStream1 {
    run(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}


fn run(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse2::<Input>(input)?;
    // println!("{:#?}", input);

    let mut match_arms = Vec::new();
    let mut asset_defs = Vec::new();

    for (path, asset) in input.serve {
        let idx = match_arms.len();
        match_arms.push(quote! {
            #path => Some(#idx),
        });

        let hash = asset.hash;
        let template = asset.mods.template;
        let append = match asset.mods.append {
            Some(s) => quote! { Some(#s) },
            None => quote! { None },
        };
        let prepend = match asset.mods.prepend {
            Some(s) => quote! { Some(#s) },
            None => quote! { None },
        };
        let content_field = match cfg!(debug_assertions) {
            true => quote! {},
            false => quote! { content: include_bytes!(#path) },
        };

        asset_defs.push(quote! {
            reinda::AssetDef {
                path: #path,
                serve: true,
                hash: #hash,
                template: #template,
                append: #append,
                prepend: #prepend,
                #content_field
            }
        });
    }

    Ok(quote! { reinda::Setup {
        assets: &[#( #asset_defs ,)*],
        path_to_idx: |s: &str| -> Option<usize> {
            match s {
                #( #match_arms )*
                _ => None,
            }
        },
    } })
}

#[derive(Debug)]
struct Input {
    serve: HashMap<String, ServedAsset>,
    includes: HashMap<String, IncludedAsset>,
}

#[derive(Debug)]
struct ServedAsset {
    hash: bool,
    mods: Modifications,
}

#[derive(Debug)]
struct IncludedAsset {
    mods: Modifications,
}

#[derive(Debug)]
struct Modifications {
    template: bool,
    append: Option<String>,
    prepend: Option<String>,
}

impl Default for ServedAsset {
    fn default() -> Self {
        Self {
            hash: false,
            mods: Modifications::default(),
        }
    }
}

impl Default for IncludedAsset {
    fn default() -> Self {
        Self {
            mods: Modifications::default(),
        }
    }
}

impl Default for Modifications {
    fn default() -> Self {
        Self {
            template: false,
            append: None,
            prepend: None,
        }
    }
}
