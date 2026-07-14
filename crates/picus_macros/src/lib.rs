//! Procedural macros for Picus application components.
//!
//! Prefer importing these through the `picus` facade (`#[derive(UiComponent)]`,
//! [`ui_view`]).

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::{
    DeriveInput, Ident, ItemFn, LitStr, Path, Result, Token, parse::Parse, parse::ParseStream,
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

/// Attribute form of [`UiComponentAttrs`] for `#[ui_view(...)]`.
impl UiComponentAttrs {
    fn parse_attr_tokens(input: ParseStream<'_>) -> Result<Self> {
        let mut attrs = Self::default();
        if input.is_empty() {
            return Ok(attrs);
        }
        let items = Punctuated::<UiViewAttrItem, Token![,]>::parse_terminated(input)?;
        for item in items {
            match item {
                UiViewAttrItem::Resources(paths) => attrs.resources.extend(paths),
                UiViewAttrItem::StyleName(name) => attrs.style_name = Some(name),
                UiViewAttrItem::RuntimeOnly => attrs.runtime_only = true,
            }
        }
        Ok(attrs)
    }
}

enum UiViewAttrItem {
    Resources(Vec<Path>),
    StyleName(LitStr),
    RuntimeOnly,
}

impl Parse for UiViewAttrItem {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident: Ident = input.parse()?;
        if ident == "resources" {
            let content;
            syn::parenthesized!(content in input);
            let paths = Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
            Ok(Self::Resources(paths.into_iter().collect()))
        } else if ident == "style_name" {
            input.parse::<Token![=]>()?;
            Ok(Self::StyleName(input.parse()?))
        } else if ident == "runtime_only" {
            Ok(Self::RuntimeOnly)
        } else {
            Err(syn::Error::new(
                ident.span(),
                "unsupported ui_view attribute (expected resources / style_name / runtime_only)",
            ))
        }
    }
}

/// Turn a function into a zero-sized `UiComponent` + `UiComponentTemplate`.
///
/// ```ignore
/// #[ui_view(resources(Count))]
/// fn CountLabel(ctx: ProjectionCtx<'_>) -> UiView {
///     let n = ctx.world.resource::<Count>().0;
///     Arc::new(label(format!("{n}")))
/// }
/// ```
///
/// Expands to a `Component + Clone + Default` struct with the function name,
/// implements `UiComponentTemplate::project` with the function body, and
/// implements registration metadata (same as `#[derive(UiComponent)]`).
///
/// Register with `register_ui_components!(app, CountLabel)`.
#[proc_macro_attribute]
pub fn ui_view(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_ui_view(attr, item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_ui_view(attr: TokenStream, item: TokenStream) -> Result<proc_macro2::TokenStream> {
    let attrs = syn::parse::Parser::parse(UiComponentAttrs::parse_attr_tokens, attr)?;
    let input_fn = syn::parse::<ItemFn>(item)?;

    if !input_fn.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input_fn.sig.generics,
            "#[ui_view] does not support generic functions",
        ));
    }
    if input_fn.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &input_fn.sig.inputs,
            "#[ui_view] function must take a single `ctx: ProjectionCtx<'_>` parameter",
        ));
    }

    let vis = &input_fn.vis;
    let name = &input_fn.sig.ident;
    let body = &input_fn.block;
    let picus = picus_crate_path();

    let ctx_pat = match input_fn.sig.inputs.first() {
        Some(syn::FnArg::Typed(pat)) => &pat.pat,
        _ => {
            return Err(syn::Error::new_spanned(
                &input_fn.sig,
                "#[ui_view] expects `fn Name(ctx: ProjectionCtx<'_>) -> UiView`",
            ));
        }
    };

    let authoring_asserts = if attrs.runtime_only {
        quote! {}
    } else {
        quote! {
            const _: () = {
                fn _assert_default<T: ::core::default::Default>() {}
                fn _assert_clone<T: ::core::clone::Clone>() {}
                fn _check() {
                    _assert_default::<#name>();
                    _assert_clone::<#name>();
                }
            };
        }
    };

    let resource_regs = attrs.resources.iter().map(|path| {
        quote! {
            #picus::__macro_support::register_projection_resource::<#path>(app);
        }
    });

    let style_reg = if let Some(style_name) = &attrs.style_name {
        quote! {
            #picus::__macro_support::register_style_selector_type::<Self>(app, #style_name);
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #authoring_asserts

        #[derive(
            #picus::bevy_ecs::prelude::Component,
            ::core::clone::Clone,
            ::core::default::Default,
            ::core::fmt::Debug,
        )]
        #vis struct #name;

        impl #picus::components::UiComponentTemplate for #name {
            fn project(_component: &Self, #ctx_pat: #picus::ProjectionCtx<'_>) -> #picus::UiView {
                #body
            }
        }

        impl #picus::__macro_support::UiComponentRegistration for #name {
            fn register(app: &mut #picus::bevy_app::App) {
                #picus::__macro_support::register_ui_component::<Self>(app);
                #(#resource_regs)*
                #style_reg
            }
        }
    })
}
