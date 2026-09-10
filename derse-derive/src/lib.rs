//! Derive serialization and deserialization for structs and enums.

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

/// Derives `Serialize`, preserving field order and prefixing the encoded body with its length.
#[proc_macro_derive(Serialize, attributes(derse))]
pub fn derse_serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_serialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives `Deserialize` and `DetailedDeserialize`.
///
/// Missing trailing fields use `Default` unless marked `#[derse(required)]` or
/// `#[derse(default = "path::to_function")]`. Required fields always invoke their
/// deserializer; custom default functions take no arguments and return the field type.
#[proc_macro_derive(Deserialize, attributes(derse))]
pub fn derse_deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_deserialize(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

enum DefaultPolicy {
    Trait,
    Required,
    Function(Path),
}

fn default_policy(field: &Field) -> syn::Result<DefaultPolicy> {
    let mut policy = None;
    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derse"))
    {
        attr.parse_nested_meta(|meta| {
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
    Ok(policy.unwrap_or(DefaultPolicy::Trait))
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

struct ValidatedInput<'a> {
    data: SupportedData<'a>,
    fields: Vec<&'a Field>,
    policies: Vec<DefaultPolicy>,
}

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
        .map(|field| default_policy(field))
        .collect::<syn::Result<_>>()?;
    Ok(ValidatedInput {
        data,
        fields,
        policies,
    })
}

// Include higher-ranked lifetimes and paths inside custom-default strings.
// Collect once per derive rather than rescanning the input for every binding.
#[derive(Default)]
struct Names(HashSet<String>);

impl<'ast> Visit<'ast> for Names {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.0.insert(ident.unraw().to_string());
    }
}

impl Names {
    fn new(input: &DeriveInput, policies: &[DefaultPolicy]) -> Self {
        let mut names = Self::default();
        names.visit_derive_input(input);
        for policy in policies {
            if let DefaultPolicy::Function(path) = policy {
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

// Concrete fields need no where predicates. In particular, adding them for
// recursive aliases or mutually recursive types would create a trait-bound cycle.
fn uses_generics(ty: &Type, generics: &Generics) -> bool {
    let mut names = Names::default();
    names.visit_type(ty);
    generics.params.iter().any(|param| {
        let ident = match param {
            GenericParam::Type(param) => &param.ident,
            GenericParam::Const(param) => &param.ident,
            GenericParam::Lifetime(param) => &param.lifetime.ident,
        };
        names.0.contains(&ident.unraw().to_string())
    })
}

// Preserve explicit user bounds, including the input lifetime they name. This
// is also important for generic recursive aliases, which cannot be recognized
// from their spelling in the derive input.
fn bounds_for<'a>(generics: &'a Generics, ident: &Ident) -> Vec<&'a syn::TypeParamBound> {
    let mut bounds: Vec<_> = generics
        .type_params()
        .filter(|param| param.ident == *ident)
        .flat_map(|param| &param.bounds)
        .collect();
    if let Some(clause) = &generics.where_clause {
        for predicate in &clause.predicates {
            if let syn::WherePredicate::Type(predicate) = predicate {
                if matches!(&predicate.bounded_ty, Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident(ident))
                {
                    bounds.extend(&predicate.bounds);
                }
            }
        }
    }
    bounds
}

fn deserialize_lifetime(bound: &syn::TypeParamBound) -> Option<&Lifetime> {
    let syn::TypeParamBound::Trait(bound) = bound else {
        return None;
    };
    if bound.lifetimes.is_some() {
        return None;
    }
    let segment = bound.path.segments.last()?;
    if segment.ident != "Deserialize" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Lifetime(lifetime) => Some(lifetime),
        _ => None,
    })
}

fn existing_input_lifetime(generics: &Generics) -> Option<Lifetime> {
    generics
        .type_params()
        .flat_map(|param| bounds_for(generics, &param.ident))
        .filter_map(deserialize_lifetime)
        .find(|lifetime| {
            generics
                .lifetimes()
                .any(|param| param.lifetime == **lifetime)
        })
        .cloned()
}

fn explicitly_bounded(
    ty: &Type,
    generics: &Generics,
    trait_name: &str,
    input: Option<&Lifetime>,
) -> bool {
    let mut used = Names::default();
    used.visit_type(ty);
    let parameters: Vec<_> = generics
        .type_params()
        .filter(|param| used.0.contains(&param.ident.unraw().to_string()))
        .collect();
    if parameters.is_empty()
        || generics
            .const_params()
            .any(|param| used.0.contains(&param.ident.unraw().to_string()))
    {
        return false;
    }
    // T: Trait says nothing about T::Associated. Keep precise field bounds for
    // projections, and for output lifetimes independent of an explicit input.
    struct Projection {
        parameters: HashSet<String>,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Projection {
        fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
            if ty.qself.is_some()
                || (ty.path.segments.len() > 1
                    && ty
                        .path
                        .segments
                        .first()
                        .is_some_and(|s| self.parameters.contains(&s.ident.unraw().to_string())))
            {
                self.found = true;
            }
            syn::visit::visit_type_path(self, ty);
        }
    }
    let mut projection = Projection {
        parameters: parameters
            .iter()
            .map(|param| param.ident.unraw().to_string())
            .collect(),
        found: false,
    };
    projection.visit_type(ty);
    if projection.found
        || input.is_some_and(|input| {
            generics.lifetimes().any(|param| {
                param.lifetime != *input
                    && used.0.contains(&param.lifetime.ident.unraw().to_string())
            })
        })
    {
        return false;
    }
    parameters.iter().all(|param| {
        bounds_for(generics, &param.ident).into_iter().any(|bound| {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return false;
            };
            let compatible_lifetime = match input {
                Some(input) => {
                    trait_bound.lifetimes.is_some() || deserialize_lifetime(bound) == Some(input)
                }
                None => true,
            };
            compatible_lifetime
                && trait_bound
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == trait_name)
        })
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

// Tag each field's bounds so that types differing only in lifetime cannot cause
// ambiguous predicates. Tuple and PhantomData impls already exist in derse;
// the marker encodes no bytes and avoids adding a new runtime API dependency.
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
        if !uses_generics(ty, &input.generics)
            || explicitly_bounded(ty, &input.generics, "Serialize", None)
        {
            continue;
        }
        let marker = marker(index);
        if contains_self(ty, name) {
            for ty in nonrecursive_types(ty, name) {
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
    let write = |value: TokenStream2, index: usize| {
        let marker = marker(index);
        quote! {
            #krate::Serialize::serialize_to(
                &(#value, <#marker as ::core::default::Default>::default()), #serializer,
            )?;
        }
    };
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
    policies: &[DefaultPolicy],
) -> TokenStream2 {
    let values = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            let marker = marker(*index);
            let policy = &policies[*index];
            *index += 1;
            // The tuple bound also avoids conflicting with an explicit
            // T: Deserialize<'a> that the user may have put on their type.
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
    let existing_lifetime = existing_input_lifetime(&input.generics);
    let lifetime = existing_lifetime
        .clone()
        .unwrap_or_else(|| names.lifetime("__derse_de"));
    let mut generics = input.generics.clone();
    if existing_lifetime.is_none() {
        generics.params.insert(
            0,
            GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
        );
    }
    for (index, field) in fields.iter().enumerate() {
        let ty = &field.ty;
        if !uses_generics(ty, &input.generics) {
            continue;
        }
        let marker = marker(index);
        if !explicitly_bounded(ty, &input.generics, "Deserialize", Some(&lifetime)) {
            if contains_self(ty, name) {
                for ty in nonrecursive_types(ty, name) {
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
        if matches!(policies[index], DefaultPolicy::Trait)
            && !explicitly_bounded(ty, &input.generics, "Default", None)
        {
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
            quote! {
                let #tag = <::std::borrow::Cow<#lifetime, str> as #krate::Deserialize<#lifetime>>::deserialize_from(#buf)?;
                match #tag.as_ref() {
                    #(#arms,)*
                    _ => ::core::result::Result::Err(#krate::Error::InvalidType(
                        ::std::format!("{}::{}", #name, #tag),
                    )),
                }
            }
        }
    };
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
    // Do not mistake associated types such as T::Node or other::Node<T>
    // for a recursive reference to the type being derived.
    path.segments.first().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "crate" | "self" | "super"
        )
    }) && path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == *name)
}

fn contains_self(ty: &Type, name: &Ident) -> bool {
    struct ContainsSelf<'a> {
        name: &'a Ident,
        found: bool,
    }
    impl<'ast> Visit<'ast> for ContainsSelf<'_> {
        fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
            if is_self_path(ty, self.name) {
                self.found = true;
            } else {
                syn::visit::visit_type_path(self, ty);
            }
        }
    }
    let mut visitor = ContainsSelf { name, found: false };
    visitor.visit_type(ty);
    visitor.found
}

// A bound such as Vec<Node<T>>: Serialize would require the very impl being
// generated. Descend into recursive fields and bound only their nonrecursive parts.
fn nonrecursive_types<'a>(ty: &'a Type, name: &Ident) -> Vec<&'a Type> {
    struct Bounds<'a, 'ast> {
        name: &'a Ident,
        types: Vec<&'ast Type>,
    }
    impl<'ast> Visit<'ast> for Bounds<'_, 'ast> {
        fn visit_type(&mut self, ty: &'ast Type) {
            if !contains_self(ty, self.name) {
                self.types.push(ty);
            } else if !matches!(ty, Type::Path(path) if is_self_path(path, self.name)) {
                syn::visit::visit_type(self, ty);
            }
        }
    }
    let mut bounds = Bounds {
        name,
        types: Vec::new(),
    };
    bounds.visit_type(ty);
    bounds.types
}

fn get_crate_name() -> syn::Result<TokenStream2> {
    let found = proc_macro_crate::crate_name("derse")
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    Ok(match found {
        proc_macro_crate::FoundCrate::Itself => quote! { crate },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
    })
}
