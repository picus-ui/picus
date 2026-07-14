//! Procedural macros for Picus application components.
//!
//! Prefer importing these through the `picus` facade (`#[derive(UiComponent)]`).

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::{
    DeriveInput, Ident, LitStr, Path, Result, Token, parse::Parse, parse::ParseStream,
    parse_macro_input, punctuated::Punctuated,
};

fn picus_crate_path() -> proc_macro2::TokenStream {
    match crate_name("picus") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::picus),
    }
}

struct UiComponentAttrs {
    resources: Vec<Path>,
    style_name: Option<LitStr>,
    runtime_only: bool,
}

impl Default for UiComponentAttrs {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            style_name: None,
            runtime_only: false,
        }
    }
}

impl UiComponentAttrs {
    fn parse_from(input: &DeriveInput) -> Result<Self> {
        let mut attrs = Self::default();
        for attr in &input.attrs {
            if !attr.path().is_ident("ui_component") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("resources") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let paths = Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    attrs.resources.extend(paths);
                    Ok(())
                } else if meta.path.is_ident("style_name") {
                    let value = meta.value()?;
                    attrs.style_name = Some(value.parse()?);
                    Ok(())
                } else if meta.path.is_ident("runtime_only") {
                    attrs.runtime_only = true;
                    Ok(())
                } else {
                    Err(meta.error("unsupported ui_component attribute"))
                }
            })?;
        }
        Ok(attrs)
    }
}

/// Derive Picus registration metadata for a UI component.
///
/// Does **not** implement [`UiComponentTemplate`]; you still write `project` by hand.
///
/// # Attributes
///
/// - `#[ui_component(resources(Count, Draft))]` — register projection resource deps
/// - `#[ui_component(style_name = "todo.item")]` — selector type alias
/// - `#[ui_component(runtime_only)]` — skip Default + Clone authoring assertions
#[proc_macro_derive(UiComponent, attributes(ui_component))]
pub fn derive_ui_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_ui_component(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_ui_component(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let attrs = UiComponentAttrs::parse_from(input)?;
    let picus = picus_crate_path();

    let authoring_asserts = if attrs.runtime_only {
        quote! {}
    } else {
        quote! {
            const _: () = {
                fn _assert_default<T: ::core::default::Default>() {}
                fn _assert_clone<T: ::core::clone::Clone>() {}
                fn _check() {
                    _assert_default::<#name #ty_generics>();
                    _assert_clone::<#name #ty_generics>();
                }
            };
        }
    };

    let resource_regs = attrs.resources.iter().map(|path| {
        quote! {
            #picus::__macro_support::register_projection_resource::<#path>(app);
        }
    });

    let style_reg = if let Some(style_name) = attrs.style_name {
        quote! {
            #picus::__macro_support::register_style_selector_type::<Self>(app, #style_name);
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #authoring_asserts

        impl #impl_generics #picus::__macro_support::UiComponentRegistration for #name #ty_generics
        #where_clause
        {
            fn register(app: &mut #picus::bevy_app::App) {
                #picus::__macro_support::register_ui_component::<Self>(app);
                #(#resource_regs)*
                #style_reg
            }
        }
    })
}

/// Parse helper unused externally; kept for attribute expansion clarity.
#[allow(dead_code)]
struct IdentList {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for IdentList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            idents: Punctuated::parse_terminated(input)?,
        })
    }
}
