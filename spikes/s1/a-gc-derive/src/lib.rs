//! `#[derive(Trace)]` for spike S1, Pattern A (`s1-a-gc`).
//!
//! The expansion has two parts.
//!
//! 1. **`unsafe impl s1_a_gc::Trace`.**
//!    - `trace` destructures `self` exhaustively and traces every field, so a new
//!      field can never be silently left untraced.
//!    - A field marked `#[trace(require_static)]` is skipped. In exchange, the
//!      compiler checks that its type is `'static`. A `'static` type cannot hold a
//!      `Gc<'gc, _>`, because the impl has to hold for every `'gc`.
//! 2. **A compile-time proof that the type has no `Drop` impl** (`s1-a-gc`'s
//!    invariant I4). The sweep drops values in arbitrary order, so a `Drop` impl
//!    could dereference a `Gc` whose target was already freed. This is the same
//!    trick `pin-project` and `gc-arena` use: a blanket impl over `T: Drop` that
//!    would conflict with the type's own impl.
//!
//! # Unsafe boundary
//!
//! This crate contains no `unsafe` code, but it *emits* `unsafe impl` into the
//! deriving crate. There, `#![forbid(unsafe_code)]` cannot see it (language
//! evaluation §15.3, rule 7). That is why the crate is registered as
//! `emits-unsafe` in `tools/unsafe-policy.txt` and audited as part of Pattern A's
//! GC boundary.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Field, Fields, GenericParam, Ident, Type, parse_macro_input, parse_quote,
};

/// Derives `s1_a_gc::Trace`. See the crate docs for the expansion.
#[proc_macro_derive(Trace, attributes(trace))]
pub fn derive_trace(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// One field of the value being traced: its binding in the destructuring
/// pattern, its type, and whether it is skipped as `'static`.
struct Binding<'a> {
    ident: Ident,
    ty: &'a Type,
    require_static: bool,
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(param) = param {
            param.bounds.push(parse_quote!(::s1_a_gc::Trace));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut traced_types: Vec<&Type> = Vec::new();
    let mut any_static = false;
    let body = match &input.data {
        Data::Struct(data) => {
            let (pattern, bindings) = bind_fields(&data.fields)?;
            collect(&bindings, &mut traced_types, &mut any_static);
            let statements = trace_statements(&bindings);
            quote! {
                let Self #pattern = self;
                #statements
            }
        }
        Data::Enum(data) => {
            let mut arms = Vec::with_capacity(data.variants.len());
            for variant in &data.variants {
                let (pattern, bindings) = bind_fields(&variant.fields)?;
                collect(&bindings, &mut traced_types, &mut any_static);
                let statements = trace_statements(&bindings);
                let variant = &variant.ident;
                arms.push(quote! { Self::#variant #pattern => { #statements } });
            }
            quote! { match self { #(#arms)* } }
        }
        Data::Union(data) => {
            return Err(syn::Error::new(
                data.union_token.span,
                "`Trace` cannot be derived for unions: the active field is unknown",
            ));
        }
    };

    let tracer = if traced_types.is_empty() {
        quote!(_tracer)
    } else {
        quote!(__tracer)
    };
    // Compile-time proof that a skipped field cannot contain a `Gc`.
    let static_check = any_static.then(|| {
        quote! {
            fn __require_static<T: ?::core::marker::Sized + 'static>(_: &T) {}
        }
    });

    Ok(quote! {
        #[automatically_derived]
        unsafe impl #impl_generics ::s1_a_gc::Trace for #name #ty_generics #where_clause {
            const NEEDS_TRACE: bool = false
                #(|| <#traced_types as ::s1_a_gc::Trace>::NEEDS_TRACE)*;

            #[inline]
            fn trace(&self, #tracer: &mut ::s1_a_gc::Tracer<'_>) {
                #static_check
                #body
            }
        }

        const _: () = {
            trait MustNotImplDrop {}
            #[allow(drop_bounds)]
            impl<T: ::core::ops::Drop> MustNotImplDrop for T {}
            impl #impl_generics MustNotImplDrop for #name #ty_generics #where_clause {}
        };
    })
}

/// A destructuring pattern for `fields`, plus one binding per field.
fn bind_fields(fields: &Fields) -> syn::Result<(TokenStream2, Vec<Binding<'_>>)> {
    let bindings = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            Ok(Binding {
                ident: format_ident!("__field{i}"),
                ty: &field.ty,
                require_static: require_static(field)?,
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let idents = bindings.iter().map(|b| &b.ident);
    let pattern = match fields {
        Fields::Named(named) => {
            let field_names = named.named.iter().map(|f| &f.ident);
            quote!({ #(#field_names: #idents),* })
        }
        Fields::Unnamed(_) => quote!(( #(#idents),* )),
        Fields::Unit => quote!(),
    };
    Ok((pattern, bindings))
}

/// Whether the field carries `#[trace(require_static)]`. Unknown options are errors.
fn require_static(field: &Field) -> syn::Result<bool> {
    let mut found = false;
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("trace")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("require_static") {
                found = true;
                Ok(())
            } else {
                Err(meta.error("unknown `trace` option; expected `require_static`"))
            }
        })?;
    }
    Ok(found)
}

fn collect<'a>(bindings: &[Binding<'a>], traced: &mut Vec<&'a Type>, any_static: &mut bool) {
    for binding in bindings {
        if binding.require_static {
            *any_static = true;
        } else {
            traced.push(binding.ty);
        }
    }
}

fn trace_statements(bindings: &[Binding<'_>]) -> TokenStream2 {
    let statements = bindings.iter().map(|b| {
        let ident = &b.ident;
        if b.require_static {
            quote!(__require_static(#ident);)
        } else {
            quote!(::s1_a_gc::Trace::trace(#ident, __tracer);)
        }
    });
    quote!(#(#statements)*)
}
