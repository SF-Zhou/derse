//! Derive macros for the `derse` binary serialization traits.
//!
//! Applications normally use the macros re-exported by `derse`; a separate
//! `derse-derive` dependency is unnecessary. Named, tuple, and unit structs and
//! enum variants are supported. Unions are not supported.
//!
//! # Encoding and compatibility
//!
//! Each derived value starts with a `VarInt64` byte length for its body. Struct
//! bodies contain fields in declaration order. Enum bodies contain the variant
//! name encoded as a string, followed by that variant's fields in declaration
//! order. Field names are not encoded; renaming a variant changes its tag.
//!
//! Deserialization reads within the declared body length and skips any trailing
//! bytes that the known fields do not consume. Appending fields can therefore
//! preserve compatibility: older readers skip them, and newer readers can use
//! defaults when reading an older body. Reordering fields or changing their
//! encodings does not provide the same guarantee.
//!
//! # Missing fields
//!
//! By default, a field uses `Default::default()` when the remaining body is empty.
//! Field attributes can select a different policy:
//!
//! - `#[derse(default)]` explicitly selects the default behavior.
//! - `#[derse(default = "path::to_function")]` calls a zero-argument function that
//!   returns the field's type. The field does not need to implement `Default`.
//! - `#[derse(required)]` always calls the field's deserializer, without a default.
//!
//! These attributes do not change serialization. They cannot be combined on one
//! field or applied to a container or variant. Defaults do not recover from a
//! partially encoded field: if any body bytes remain, decoding errors propagate.
//! A required field whose encoding consumes no bytes can still decode from an
//! empty body.
//!
//! ```
//! use derse::{Deserialize, DownwardBytes, Serialize};
//!
//! #[derive(Serialize)]
//! struct Earlier {
//!     name: String,
//! }
//!
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct Current<'a> {
//!     #[derse(required)]
//!     name: &'a str,
//!     #[derse(default = "default_revision")]
//!     revision: u8,
//! }
//!
//! fn default_revision() -> u8 {
//!     1
//! }
//!
//! let bytes: DownwardBytes = Earlier { name: "example".into() }.serialize()?;
//! let value = Current::deserialize(bytes.as_slice())?;
//! assert_eq!(value, Current { name: "example", revision: 1 });
//! # Ok::<(), derse::Error>(())
//! ```
//!
//! # Generics and borrowed data
//!
//! The macros infer bounds from field types and preserve the input's generic
//! parameters and `where` clauses. For example, a `PhantomData<T>` field does not
//! require `T: Serialize`. Borrowed fields are supported when their own
//! `Deserialize` implementations can borrow from the chosen input deserializer.
//! The generated input lifetime is independent of the type's lifetimes. Existing
//! trait bounds are preserved; they do not suppress bounds on complete field types.
//!
//! # Recursive fields
//!
//! Recursion written directly as `Self` or the unqualified type name is detected
//! automatically. The macro cannot resolve type aliases or qualified module paths.
//! Mark a field with `#[derse(recursive)]` when such a path hides generic recursion:
//!
//! ```
//! use derse::{Deserialize, DownwardBytes, Serialize};
//!
//! type Children<T> = Vec<Node<T>>;
//!
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct Node<T: Serialize + for<'a> Deserialize<'a> + Default> {
//!     value: T,
//!     #[derse(recursive)]
//!     children: Children<T>,
//! }
//!
//! let value = Node { value: 7u8, children: Vec::new() };
//! let bytes: DownwardBytes = value.serialize()?;
//! assert_eq!(Node::<u8>::deserialize(bytes.as_slice())?, value);
//! # Ok::<(), derse::Error>(())
//! ```
//!
//! This attribute skips automatic `Serialize` and `Deserialize` bounds for the
//! whole field. Supply any required parameter bounds yourself, including bounds
//! on nonrecursive parts of a field such as `(Children<T>, U)`. Its default policy
//! is unchanged: `Default` is still required unless `required` or a custom default
//! is selected. `recursive` can be combined with any one default policy.
//!
//! Generic recursive aliases that previously relied on explicit parameter bounds
//! to suppress field-bound inference now need this annotation. A qualified path
//! to a different type is treated as that complete field type, even if its final
//! name matches the type being derived. These changes do not affect encoded bytes.
//!
//! Generated calls use the `derse` traits explicitly, so similarly named inherent
//! methods do not affect the encoding. The runtime dependency is resolved from
//! the caller's Cargo manifest, including a renamed `derse` dependency.

use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt, parse_macro_input, parse_quote, spanned::Spanned, visit::Visit, Attribute, Data,
    DeriveInput, Field, Fields, GenericParam, Generics, Lifetime, LifetimeParam, Member, Path,
    Type,
};

#[cfg(test)]
mod tests;

/// Implements `derse::Serialize` for a struct or enum.
///
/// The body preserves field declaration order and has a `VarInt64` byte-length
/// prefix. An enum body begins with its variant name encoded as a string. Field
/// default attributes are validated but do not affect the encoded bytes.
#[proc_macro_derive(Serialize, attributes(derse))]
pub fn derse_serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_serialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `derse::Deserialize` and `derse::DetailedDeserialize`.
///
/// The decoder confines field reads to the length-prefixed body and skips unknown
/// trailing bytes. Missing fields use `Default` unless marked
/// `#[derse(required)]` or `#[derse(default = "path::to_function")]`; see the
/// crate documentation for these policies. A partially encoded field remains an
/// error. `DetailedDeserialize` exposes the length and body decoding separately.
#[proc_macro_derive(Deserialize, attributes(derse))]
pub fn derse_deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_deserialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

// Each field has exactly one policy, including fields with no attribute.
enum DefaultPolicy {
    Trait,
    Required,
    Function(Path),
}

struct FieldPolicy {
    default: DefaultPolicy,
    recursive: bool,
}

fn field_policy(field: &Field) -> syn::Result<FieldPolicy> {
    let mut policy = None;
    let mut recursive = false;
    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derse"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("recursive") {
                if recursive {
                    return Err(meta.error("duplicate recursive field attribute"));
                }
                recursive = true;
                return Ok(());
            }
            let value = if meta.path.is_ident("required") {
                DefaultPolicy::Required
            } else if meta.path.is_ident("default") {
                if meta.input.peek(syn::Token![=]) {
                    let path: syn::LitStr = meta.value()?.parse()?;
                    DefaultPolicy::Function(path.parse()?)
                } else {
                    DefaultPolicy::Trait
                }
            } else {
                return Err(meta.error("unsupported derse attribute"));
            };
            if policy.is_some() {
                return Err(meta.error("duplicate or conflicting field default policy"));
            }
            policy = Some(value);
            Ok(())
        })?;
    }
    Ok(FieldPolicy {
        default: policy.unwrap_or(DefaultPolicy::Trait),
        recursive,
    })
}

fn reject_container_attributes(attrs: &[Attribute]) -> syn::Result<()> {
    if let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident("derse")) {
        return Err(syn::Error::new_spanned(
            attr,
            "derse attributes are only supported on fields",
        ));
    }
    Ok(())
}

enum SupportedData<'a> {
    Struct(&'a syn::DataStruct),
    Enum(&'a syn::DataEnum),
}

// Fields and policies share a flat index across all enum variants, in declaration
// order. Bound inference and generated reads/writes must use the same indices.
struct ValidatedInput<'a> {
    data: SupportedData<'a>,
    fields: Vec<&'a Field>,
    policies: Vec<FieldPolicy>,
}

// Both derives validate the same input, even though only Deserialize uses defaults.
fn validate(input: &DeriveInput) -> syn::Result<ValidatedInput<'_>> {
    reject_container_attributes(&input.attrs)?;
    let (data, fields): (_, Vec<_>) = match &input.data {
        Data::Struct(data) => (SupportedData::Struct(data), data.fields.iter().collect()),
        Data::Enum(data) => {
            for variant in &data.variants {
                reject_container_attributes(&variant.attrs)?;
            }
            let fields = data
                .variants
                .iter()
                .flat_map(|variant| &variant.fields)
                .collect();
            (SupportedData::Enum(data), fields)
        }
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "only structs and enums are supported",
            ));
        }
    };
    let policies = fields
        .iter()
        .map(|field| field_policy(field))
        .collect::<syn::Result<_>>()?;
    Ok(ValidatedInput {
        data,
        fields,
        policies,
    })
}

// Reserve names from the complete syntax tree, including higher-ranked lifetimes
// and paths parsed from custom-default strings. Compare raw identifiers by their
// unescaped spelling so `r#__derse_buf` also reserves `__derse_buf`.
#[derive(Default)]
struct Names(HashSet<String>);

impl<'ast> Visit<'ast> for Names {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.0.insert(ident.unraw().to_string());
    }
}

impl Names {
    fn new(input: &DeriveInput, policies: &[FieldPolicy]) -> Self {
        let mut names = Self::default();
        names.visit_derive_input(input);
        for policy in policies {
            if let DefaultPolicy::Function(path) = &policy.default {
                names.visit_path(path);
            }
        }
        names
    }

    fn ident(&self, base: &str) -> Ident {
        let mut name = base.to_owned();
        while self.0.contains(&name) {
            name.push('_');
        }
        Ident::new(&name, Span::mixed_site())
    }

    fn lifetime(&self, base: &str) -> Lifetime {
        Lifetime::new(&format!("'{}", self.ident(base)), Span::mixed_site())
    }
}

// Concrete fields are checked by the generated method bodies. Adding redundant
// where predicates for them can create cycles through aliases or mutually
// recursive types. This syntactic check also includes consts and lifetimes.
fn uses_generics(ty: &Type, generics: &Generics) -> bool {
    let mut names = Names::default();
    names.visit_type(ty);
    (!generics.params.is_empty() && names.0.contains("Self"))
        || generics.params.iter().any(|param| {
            let ident = match param {
                GenericParam::Type(param) => &param.ident,
                GenericParam::Const(param) => &param.ident,
                GenericParam::Lifetime(param) => &param.lifetime.ident,
            };
            names.0.contains(&ident.unraw().to_string())
        })
}

// Mixed-site spans alone do not protect bindings from constants in the caller's
// scope. Function aliases hide those constants, so parameters and patterns are
// always interpreted as bindings. The anonymous const keeps the aliases private.
fn isolate(impls: TokenStream2, bindings: &[Ident]) -> TokenStream2 {
    quote! {
        const _: () = {
            #( #[allow(unused_imports)] use ::core::convert::identity as #bindings; )*
            #impls
        };
    }
}

fn member(index: usize, field: &Field) -> Member {
    field
        .ident
        .clone()
        .map(Member::Named)
        .unwrap_or_else(|| Member::Unnamed(index.into()))
}

fn construct(fields: &Fields, constructor: TokenStream2, values: &[TokenStream2]) -> TokenStream2 {
    match fields {
        Fields::Named(fields) => {
            let names = fields.named.iter().map(|field| &field.ident);
            quote! { #constructor { #(#names: #values,)* } }
        }
        Fields::Unnamed(_) => quote! { #constructor(#(#values,)*) },
        Fields::Unit => constructor,
    }
}

// Route field bounds and trait calls through an indexed tuple. Distinct markers
// keep obligations for types differing only in lifetime from becoming ambiguous.
// Tuple encoding adds no framing and PhantomData consumes no bytes, so this bridge
// preserves each field's wire representation using existing runtime traits.
fn marker(index: usize) -> TokenStream2 {
    quote! { ::core::marker::PhantomData<[(); #index]> }
}

fn expand_serialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let ValidatedInput {
        data,
        fields,
        policies,
    } = validate(input)?;
    let names = Names::new(input, &policies);
    let krate = get_crate_name()?;
    let name = &input.ident;
    let mut generics = input.generics.clone();
    let borrow = names.lifetime("__derse_borrow");
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        if !uses_generics(ty, &input.generics) || policies[index].recursive {
            continue;
        }
        let marker = marker(index);
        if contains_self(ty, name, &input.generics) {
            for ty in nonrecursive_types(ty, name, &input.generics) {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote!(#ty: #krate::Serialize));
            }
        } else {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(for<#borrow> (&#borrow #ty, #marker): #krate::Serialize));
        }
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let serializer_ty = names.ident("__DerseSerializer");
    let serializer = names.ident("__derse_serializer");
    let start = names.ident("__derse_start");
    let len = names.ident("__derse_len");
    // Use trait-qualified calls through the same bridge as the where predicates;
    // method syntax could select a field type's unrelated inherent method.
    let write = |value: TokenStream2, index: usize| {
        let marker = marker(index);
        quote! {
            #krate::Serialize::serialize_to(
                &(#value, <#marker as ::core::default::Default>::default()), #serializer,
            )?;
        }
    };
    // Serializers prepend, so writing fields backwards preserves declaration
    // order in the finished body. Enum tags and body lengths are prepended last.
    let statements = match &data {
        SupportedData::Struct(data) => {
            let statements = data.fields.iter().enumerate().rev().map(|(index, field)| {
                let member = member(index, field);
                write(quote_spanned! {field.span()=> &self.#member }, index)
            });
            quote! { #(#statements)* }
        }
        SupportedData::Enum(data) => {
            let mut offset = 0;
            let arms = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let tag = ident.to_string();
                let bindings: Vec<_> = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        let binding = names.ident(&format!("__derse_field_{index}"));
                        quote! { #binding }
                    })
                    .collect();
                let pattern = construct(&variant.fields, quote! { Self::#ident }, &bindings);
                let writes: Vec<_> = bindings
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, binding)| write(binding.clone(), offset + index))
                    .collect();
                offset += bindings.len();
                quote! {
                    #pattern => {
                        #(#writes)*
                        #krate::Serialize::serialize_to(#tag, #serializer)?;
                    }
                }
            });
            quote! { match self { #(#arms,)* } }
        }
    };
    // Matching an empty enum through &Self is not exhaustive; dereferencing makes
    // its uninhabited type visible to the exhaustiveness checker.
    let body = if matches!(&data, SupportedData::Enum(data) if data.variants.is_empty()) {
        quote! { match *self {} }
    } else {
        quote! {
            let #start = #krate::Serializer::len(#serializer);
            #statements
            let #len = #krate::Serializer::len(#serializer) - #start;
            #krate::Serialize::serialize_to(&#krate::VarInt64(#len as u64), #serializer)
        }
    };
    let mut bindings = vec![serializer.clone(), start.clone(), len.clone()];
    if let SupportedData::Enum(data) = &data {
        let count = data
            .variants
            .iter()
            .map(|variant| variant.fields.len())
            .max()
            .unwrap_or(0);
        bindings.extend((0..count).map(|index| names.ident(&format!("__derse_field_{index}"))));
    }
    Ok(isolate(
        quote! {
            #[automatically_derived]
            impl #impl_generics #krate::Serialize for #name #ty_generics #where_clause {
                fn serialize_to<#serializer_ty: #krate::Serializer>(
                    &self, #serializer: &mut #serializer_ty,
                ) -> #krate::Result<()> {
                    #body
                }
            }
        },
        &bindings,
    ))
}

fn deserialize_fields(
    fields: &Fields,
    constructor: TokenStream2,
    krate: &TokenStream2,
    lifetime: &Lifetime,
    buf: &Ident,
    index: &mut usize,
    policies: &[FieldPolicy],
) -> TokenStream2 {
    let values = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            let marker = marker(*index);
            let policy = &policies[*index].default;
            *index += 1;
            // Match the indexed predicates used by bound inference. The same
            // bridge delegates both decoding and Default to the field type.
            let bridge = quote! { (#ty, #marker) };
            let read = quote_spanned! {field.span()=>
                <#bridge as #krate::Deserialize<#lifetime>>::deserialize_from(#buf)?.0
            };
            let default = match policy {
                DefaultPolicy::Required => return read,
                DefaultPolicy::Trait => quote_spanned! {field.span()=>
                    <#bridge as ::core::default::Default>::default().0
                },
                DefaultPolicy::Function(path) => quote_spanned! {field.span()=> #path() },
            };
            // Defaults describe absent trailing fields, never recovery from a
            // field decoder's error. Required fields bypass this emptiness test.
            quote! {
                if #krate::Deserializer::is_empty(#buf) { #default } else { #read }
            }
        })
        .collect::<Vec<_>>();
    let value = construct(fields, constructor, &values);
    quote! { ::core::result::Result::Ok(#value) }
}

fn expand_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let ValidatedInput {
        data,
        fields,
        policies,
    } = validate(input)?;
    let names = Names::new(input, &policies);
    let krate = get_crate_name()?;
    let name = &input.ident;
    let lifetime = names.lifetime("__derse_de");
    let mut generics = input.generics.clone();
    generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
    );
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        if !uses_generics(ty, &input.generics) {
            continue;
        }
        let marker = marker(index);
        if !policies[index].recursive {
            if contains_self(ty, name, &input.generics) {
                for ty in nonrecursive_types(ty, name, &input.generics) {
                    generics
                        .make_where_clause()
                        .predicates
                        .push(parse_quote!(#ty: #krate::Deserialize<#lifetime>));
                }
            } else {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote!((#ty, #marker): #krate::Deserialize<#lifetime>));
            }
        }
        if matches!(policies[index].default, DefaultPolicy::Trait) {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!((#ty, #marker): ::core::default::Default));
        }
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let deserializer_ty = names.ident("__DerseDeserializer");
    let buf = names.ident("__derse_buf");
    let len = names.ident("__derse_len");
    let tag = names.ident("__derse_tag");
    let mut index = 0;
    let body = match data {
        SupportedData::Struct(data) => deserialize_fields(
            &data.fields,
            quote! { Self },
            &krate,
            &lifetime,
            &buf,
            &mut index,
            &policies,
        ),
        SupportedData::Enum(data) => {
            let name = name.to_string();
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let tag = ident.to_string();
                    let value = deserialize_fields(
                        &variant.fields,
                        quote! { Self::#ident },
                        &krate,
                        &lifetime,
                        &buf,
                        &mut index,
                        &policies,
                    );
                    quote! { #tag => #value }
                })
                .collect::<Vec<_>>();
            // A fragmented deserializer may need to own the variant name;
            // borrowing it unconditionally as &str would reject valid input.
            quote! {
                let #tag = <::std::borrow::Cow<#lifetime, str> as #krate::Deserialize<#lifetime>>::deserialize_from(#buf)?;
                match &*#tag {
                    #(#arms,)*
                    _ => ::core::result::Result::Err(#krate::Error::InvalidType(
                        ::std::format!("{}::{}", #name, #tag),
                    )),
                }
            }
        }
    };
    // advance() consumes the whole framed body from the outer input up front.
    // Field reads cannot spill into the next value, and unread trailing fields
    // are discarded with the body view, including when a field returns an error.
    Ok(isolate(
        quote! {
            #[automatically_derived]
            impl #impl_generics #krate::DetailedDeserialize<#lifetime> for #name #ty_generics #where_clause {
                fn deserialize_len<#deserializer_ty: #krate::Deserializer<#lifetime>>(
                    #buf: &mut #deserializer_ty,
                ) -> #krate::Result<usize> {
                    ::core::result::Result::Ok(
                        <#krate::VarInt64 as #krate::Deserialize<#lifetime>>::deserialize_from(#buf)?.0 as usize
                    )
                }

                fn deserialize_fields<#deserializer_ty: #krate::Deserializer<#lifetime>>(
                    #buf: &mut #deserializer_ty,
                ) -> #krate::Result<Self> {
                    #body
                }
            }

            #[automatically_derived]
            impl #impl_generics #krate::Deserialize<#lifetime> for #name #ty_generics #where_clause {
                fn deserialize_from<#deserializer_ty: #krate::Deserializer<#lifetime>>(
                    #buf: &mut #deserializer_ty,
                ) -> #krate::Result<Self> {
                    let #len = <Self as #krate::DetailedDeserialize<#lifetime>>::deserialize_len(#buf)?;
                    let mut #buf = #krate::Deserializer::advance(#buf, #len)?;
                    <Self as #krate::DetailedDeserialize<#lifetime>>::deserialize_fields(&mut #buf)
                }
            }
        },
        &[buf, len, tag],
    ))
}

// Recursion detection is deliberately syntactic: the derive has no name resolver
// for aliases or arbitrary module paths. Those need an explicit recursive flag.
fn is_self_path(ty: &syn::TypePath, name: &Ident) -> bool {
    if ty.qself.is_some() {
        return false;
    }
    let path = &ty.path;
    if path.segments.len() == 1 {
        return path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "Self" || segment.ident == *name);
    }
    false
}

fn contains_self(ty: &Type, name: &Ident, generics: &Generics) -> bool {
    struct ContainsSelf<'a> {
        name: &'a Ident,
        generics: &'a Generics,
        found: bool,
    }
    impl<'ast> Visit<'ast> for ContainsSelf<'_> {
        fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
            if is_self_path(ty, self.name) {
                self.found = true;
            } else if ty.qself.is_none()
                && !ty.path.segments.first().is_some_and(|segment| {
                    ty.path.segments.len() > 1
                        && (segment.ident == "Self"
                            || self
                                .generics
                                .type_params()
                                .any(|param| param.ident == segment.ident))
                })
            {
                // Associated types are opaque, including Self arguments of a
                // GAT. The projection may produce an entirely unrelated type.
                syn::visit::visit_type_path(self, ty);
            }
        }
    }
    let mut visitor = ContainsSelf {
        name,
        generics,
        found: false,
    };
    visitor.visit_type(ty);
    visitor.found
}

// A bound such as Vec<Node<T>>: Serialize would require the very impl being
// generated. Descend into recursive fields and bound only their nonrecursive parts.
fn nonrecursive_types<'a>(ty: &'a Type, name: &Ident, generics: &Generics) -> Vec<&'a Type> {
    struct Bounds<'a, 'ast> {
        name: &'a Ident,
        generics: &'a Generics,
        types: Vec<&'ast Type>,
    }
    impl<'ast> Visit<'ast> for Bounds<'_, 'ast> {
        fn visit_type(&mut self, ty: &'ast Type) {
            if !contains_self(ty, self.name, self.generics) {
                self.types.push(ty);
            } else if !matches!(ty, Type::Path(path) if is_self_path(path, self.name)) {
                syn::visit::visit_type(self, ty);
            }
        }
    }
    let mut bounds = Bounds {
        name,
        generics,
        types: Vec::new(),
    };
    bounds.visit_type(ty);
    bounds.types
}

// Resolve Cargo dependency renames instead of assuming `::derse`. Missing runtime
// dependencies are expansion errors; treating them as `crate` hides the cause.
fn get_crate_name() -> syn::Result<TokenStream2> {
    let found = proc_macro_crate::crate_name("derse")
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    Ok(match found {
        proc_macro_crate::FoundCrate::Itself => quote! { crate },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = Ident::new_raw(&name, Span::call_site());
            quote! { ::#ident }
        }
    })
}
