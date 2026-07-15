use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Expr, ExprLit, ItemFn, Lit, MetaNameValue,
};

/// Attribute macro that wraps a test function in a scoped env mock.
///
/// # Example
/// ```rust,ignore
/// #[test]
/// #[tpt_env(DB_URL = "postgres://localhost/test", LOG_LEVEL = "debug")]
/// fn test_uses_env() {
///     assert_eq!(std::env::var("DB_URL").unwrap(), "postgres://localhost/test");
/// }
/// ```
#[proc_macro_attribute]
pub fn tpt_env(args: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    // Parse args as comma-separated `KEY = "value"` pairs.
    let pairs = match syn::parse::Parser::parse(
        Punctuated::<MetaNameValue, Comma>::parse_terminated,
        args,
    ) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut set_calls = vec![];

    for nv in &pairs {
        let key = match nv.path.get_ident() {
            Some(i) => i.to_string(),
            None => {
                return syn::Error::new_spanned(&nv.path, "expected a simple identifier for env var key")
                    .to_compile_error()
                    .into();
            }
        };
        let val = match &nv.value {
            Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => s.value(),
            other => {
                return syn::Error::new_spanned(other, "tpt_env values must be string literals")
                    .to_compile_error()
                    .into();
            }
        };
        set_calls.push(quote! { .set(#key, #val) });
    }

    let sig = &func.sig;
    let vis = &func.vis;
    let attrs = &func.attrs;
    let body = &func.block;
    let asyncness = &sig.asyncness;

    let expanded = if asyncness.is_some() {
        quote! {
            #(#attrs)*
            #vis #sig {
                let _guard = ::tpt_env_mocker::MockEnv::new()
                    #(#set_calls)*
                    .lock();
                let result = async move #body;
                result.await
            }
        }
    } else {
        quote! {
            #(#attrs)*
            #vis #sig {
                let _guard = ::tpt_env_mocker::MockEnv::new()
                    #(#set_calls)*
                    .lock();
                #body
            }
        }
    };

    // Suppress unused import warning on Span — it's only used in error paths.
    let _ = Span::call_site();
    expanded.into()
}
