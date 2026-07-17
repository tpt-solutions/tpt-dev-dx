//! Proc-macro internals for `tpt-fixture`. Do not use directly — depend on
//! `tpt-fixture` instead.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, FnArg, Ident, ItemFn, LitStr, Pat, ReturnType, Token, Type, TypeTuple};

/// Scope of a fixture.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Test,
    Module,
    Suite,
}

impl Scope {
    fn as_path(&self) -> proc_macro2::TokenStream {
        match self {
            Scope::Test => quote!(tpt_fixture::Scope::Test),
            Scope::Module => quote!(tpt_fixture::Scope::Module),
            Scope::Suite => quote!(tpt_fixture::Scope::Suite),
        }
    }
}

/// Parsed `#[tpt_fixture(...)]` arguments.
struct FixtureArgs {
    scope: Scope,
    name: Option<String>,
}

impl Default for FixtureArgs {
    fn default() -> Self {
        // Fixtures default to `suite` scope (initialised once, shared widely).
        FixtureArgs {
            scope: Scope::Suite,
            name: None,
        }
    }
}

impl Parse for FixtureArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = FixtureArgs::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "scope" => {
                    let s: LitStr = input.parse()?;
                    args.scope = match s.value().as_str() {
                        "test" => Scope::Test,
                        "module" => Scope::Module,
                        "suite" => Scope::Suite,
                        other => {
                            return Err(syn::Error::new(
                                s.span(),
                                format!("unknown scope `{other}` (expected test|module|suite)"),
                            ))
                        }
                    };
                }
                "name" => {
                    let s: LitStr = input.parse()?;
                    args.name = Some(s.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown fixture argument `{other}` (expected scope|name)"),
                    ))
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}

/// Heuristically detect whether an item is a test function (so the macro should
/// inject fixtures rather than define one). A function carrying any attribute
/// whose final path segment is `test` (e.g. `#[test]`, `#[tokio::test]`,
/// `#[async_std::test]`) is treated as a test.
fn is_test_fn(item: &ItemFn) -> bool {
    item.attrs.iter().any(|a| last_segment_is(a, "test"))
}

fn last_segment_is(attr: &Attribute, name: &str) -> bool {
    if let Some(seg) = attr.path().segments.last() {
        return seg.ident == name;
    }
    false
}

#[proc_macro_attribute]
pub fn tpt_fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as FixtureArgs);
    let item = syn::parse_macro_input!(input as ItemFn);

    if is_test_fn(&item) {
        expand_test(item)
    } else {
        expand_definition(args, item)
    }
    .into()
}

/// Extract the resource type `T` from a fixture init fn's return type.
///
/// A plain `T` yields `T`; a tuple `(T, teardown)` yields `T`. Anything else
/// (e.g. `-> impl Trait`) is passed through untouched and will fail with a
/// clearer error at the `fixture_access` call site.
fn resource_type_of(output: &ReturnType) -> proc_macro2::TokenStream {
    if let ReturnType::Type(_, ty) = output {
        if let Type::Tuple(TypeTuple { elems, .. }) = &**ty {
            if elems.len() == 2 {
                let first = elems.first().expect("tuple has 2 elements");
                return quote! { #first };
            }
        }
        return quote! { #ty };
    }
    // No explicit return type (`-> ()`); default to unit.
    quote! { () }
}

/// Expand a fixture *definition*: keep the init fn (renamed) and emit a public
/// accessor that resolves/creates the shared `Arc<T>`.
fn expand_definition(args: FixtureArgs, mut item: ItemFn) -> proc_macro2::TokenStream {
    let fixture_name = args
        .name
        .clone()
        .unwrap_or_else(|| item.sig.ident.to_string());
    let accessor = Ident::new(&fixture_name, item.sig.ident.span());
    let init_fn = format_ident!("__tpt_fixture_init_{}", fixture_name);
    let scope = args.scope.as_path();

    // Rename the user's init fn so it doesn't collide with the accessor.
    item.sig.ident = init_fn.clone();
    let vis = &item.vis;
    let is_async = item.sig.asyncness.is_some();

    // Derive the resource type `T` from the init fn's return type: a tuple
    // `(T, teardown)` yields `T`; a plain `T` yields `T`.
    let res_ty = resource_type_of(&item.sig.output);

    let init_expr = if is_async {
        quote! { tpt_fixture::IntoFixture::into_fixture(tpt_fixture::block_on(#init_fn())) }
    } else {
        quote! { tpt_fixture::IntoFixture::into_fixture(#init_fn()) }
    };

    let accessor_fn = quote! {
        #vis fn #accessor() -> ::std::sync::Arc<#res_ty> {
            tpt_fixture::fixture_access(#fixture_name, #scope, || #init_expr)
        }
    };

    quote! {
        #item
        #accessor_fn
    }
}

/// Expand a test *usage*: strip fixture parameters, resolve each by calling the
/// accessor of the same name, and wrap the body in a `TestScopeGuard` so
/// `test`-scope teardowns run at the end (even on panic).
fn expand_test(mut item: ItemFn) -> proc_macro2::TokenStream {
    // Collect fixture parameter idents and their types.
    let mut fixture_params: Vec<Ident> = Vec::new();
    let mut new_inputs = syn::punctuated::Punctuated::<FnArg, Token![,]>::new();
    for arg in item.sig.inputs.iter() {
        match arg {
            FnArg::Typed(pt) => {
                if let Pat::Ident(pat_ident) = &*pt.pat {
                    fixture_params.push(pat_ident.ident.clone());
                    // Drop the parameter — it will be a local binding instead.
                } else {
                    new_inputs.push(arg.clone());
                }
            }
            FnArg::Receiver(r) => new_inputs.push(FnArg::Receiver(r.clone())),
        }
    }
    item.sig.inputs = new_inputs;

    let bindings = fixture_params.iter().map(|name| {
        quote! { let #name = #name(); }
    });

    let vis = &item.vis;
    let attrs = &item.attrs;
    let sig = &item.sig;
    let block = &item.block;

    quote! {
        #(#attrs)*
        #vis #sig {
            let _tpt_scope_guard = tpt_fixture::TestScopeGuard;
            #(#bindings)*
            #block
        }
    }
}
