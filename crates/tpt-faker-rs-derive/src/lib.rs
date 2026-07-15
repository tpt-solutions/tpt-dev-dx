use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::quote;
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Data, DeriveInput, Expr, ExprLit, Fields, Lit, MetaNameValue,
};

/// Derive the `Fake` trait for a struct.
///
/// Each field can be annotated with `#[fake(...)]` to customise generation.
///
/// # Supported annotations
/// - `#[fake(kind = "name")]` — full locale-aware name
/// - `#[fake(kind = "first_name")]`
/// - `#[fake(kind = "last_name")]`
/// - `#[fake(kind = "email")]`
/// - `#[fake(kind = "username")]`
/// - `#[fake(kind = "url")]`
/// - `#[fake(kind = "ipv4")]`
/// - `#[fake(kind = "ipv6")]`
/// - `#[fake(kind = "uuid")]`
/// - `#[fake(kind = "luhn_card")]`
/// - `#[fake(kind = "iso_date")]`
/// - `#[fake(kind = "iso_datetime")]`
/// - `#[fake(kind = "word")]`
/// - `#[fake(kind = "sentence")]`
/// - `#[fake(kind = "paragraph")]`
/// - `#[fake(range = "lo..=hi")]` — bounded integer
#[proc_macro_derive(Fake, attributes(fake))]
pub fn derive_fake(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => &s.fields,
        _ => {
            return syn::Error::new_spanned(&input, "Fake can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let field_inits = match fields {
        Fields::Named(named) => {
            let mut inits = vec![];
            for f in &named.named {
                let ident = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let gen = field_generator(f);
                inits.push(quote! { #ident: (#gen) as #ty });
            }
            inits
        }
        Fields::Unit => vec![],
        Fields::Unnamed(_) => {
            return syn::Error::new_spanned(&input, "Fake does not support tuple structs yet")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl ::tpt_faker_rs::Fake for #name {
            fn fake() -> Self {
                use ::tpt_faker_rs::gen;
                Self {
                    #(#field_inits,)*
                }
            }
        }
    };

    expanded.into()
}

fn field_generator(f: &syn::Field) -> TS2 {
    for attr in &f.attrs {
        if !attr.path().is_ident("fake") {
            continue;
        }
        // Parse inner tokens as `key = "value"` pairs.
        let pairs: Punctuated<MetaNameValue, Comma> = match attr
            .parse_args_with(Punctuated::<MetaNameValue, Comma>::parse_terminated)
        {
            Ok(p) => p,
            Err(e) => return e.to_compile_error(),
        };

        for nv in &pairs {
            let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
            let val = match &nv.value {
                Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => s.value(),
                _ => continue,
            };
            match key.as_str() {
                "kind" => return kind_generator(&val),
                "range" => return range_generator(&val),
                _ => {}
            }
        }
    }
    default_generator(&f.ty)
}

fn kind_generator(kind: &str) -> TS2 {
    match kind {
        "name"         => quote! { gen::name() },
        "first_name"   => quote! { gen::first_name() },
        "last_name"    => quote! { gen::last_name() },
        "email"        => quote! { gen::email() },
        "username"     => quote! { gen::username() },
        "url"          => quote! { gen::url() },
        "ipv4"         => quote! { gen::ipv4() },
        "ipv6"         => quote! { gen::ipv6() },
        "uuid"         => quote! { gen::uuid() },
        "luhn_card"    => quote! { gen::luhn_card() },
        "iso_date"     => quote! { gen::iso_date() },
        "iso_datetime" => quote! { gen::iso_datetime() },
        "word"         => quote! { gen::word() },
        "sentence"     => quote! { gen::sentence() },
        "paragraph"    => quote! { gen::paragraph() },
        other => {
            let msg = format!("unknown fake kind: `{other}`");
            quote! { compile_error!(#msg) }
        }
    }
}

fn range_generator(range: &str) -> TS2 {
    if let Some((lo, hi)) = range.split_once("..=") {
        if let (Ok(lo), Ok(hi)) = (lo.trim().parse::<i64>(), hi.trim().parse::<i64>()) {
            return quote! { gen::range_i64(#lo, #hi) };
        }
    } else if let Some((lo, hi)) = range.split_once("..") {
        if let (Ok(lo), Ok(hi)) = (lo.trim().parse::<i64>(), hi.trim().parse::<i64>()) {
            let hi_excl = hi - 1;
            return quote! { gen::range_i64(#lo, #hi_excl) };
        }
    }
    let msg = format!("invalid range: `{range}` — expected `lo..=hi` or `lo..hi`");
    quote! { compile_error!(#msg) }
}

fn default_generator(ty: &syn::Type) -> TS2 {
    let ty_str = quote!(#ty).to_string().replace(' ', "");
    match ty_str.as_str() {
        "String" => quote! { gen::word() },
        "bool"   => quote! { gen::range_i64(0, 1) != 0 },
        "u8"     => quote! { gen::range_i64(0, 255) as u8 },
        "u16"    => quote! { gen::range_i64(0, 65535) as u16 },
        "u32"    => quote! { gen::range_i64(0, i32::MAX as i64) as u32 },
        "u64"    => quote! { gen::range_i64(0, i64::MAX) as u64 },
        "i8"     => quote! { gen::range_i64(-128, 127) as i8 },
        "i16"    => quote! { gen::range_i64(-32768, 32767) as i16 },
        "i32"    => quote! { gen::range_i64(i32::MIN as i64, i32::MAX as i64) as i32 },
        "i64"    => quote! { gen::range_i64(i64::MIN, i64::MAX) },
        "f32"    => quote! { gen::range_i64(-1000, 1000) as f32 },
        "f64"    => quote! { gen::range_i64(-1000, 1000) as f64 },
        "usize"  => quote! { gen::range_i64(0, 1000) as usize },
        _        => quote! { ::std::default::Default::default() },
    }
}
