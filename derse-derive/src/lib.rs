extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields, Lifetime, LifetimeParam,
};

#[proc_macro_derive(serialize)]
pub fn derse_serialize_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let struct_type = &ast.ident;
    let statements = match ast.data {
        Data::Struct(DataStruct { fields, .. }) => {
            let mut idents = fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let index = syn::Index::from(i);
                    f.ident
                        .as_ref()
                        .map_or(quote! {#index}, |ident| quote! {#ident})
                })
                .collect::<Vec<_>>();
            idents.reverse();
            quote! { #( self.#idents.serialize_to(serializer)?; )* }
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let mut match_statements = Vec::new();
            for variant in variants {
                let ident = &variant.ident;
                let name = ident.to_string();
                let match_statement = match variant.fields {
                    Fields::Named(fields) => {
                        let mut idents = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
                        let list = quote! { #(#idents, )* };
                        idents.reverse();
                        quote! {
                            Self::#ident { #list } => {
                                #( #idents.serialize_to(serializer)?; )*
                                #name.serialize_to(serializer)?;
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let mut idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                syn::Ident::new(&format!("v{i}"), proc_macro2::Span::call_site())
                            })
                            .collect::<Vec<_>>();
                        let list = quote! { #(#idents, )* };
                        idents.reverse();
                        quote! {
                            Self::#ident ( #list ) => {
                                #( #idents.serialize_to(serializer)?; )*
                                #name.serialize_to(serializer)?;
                            }
                        }
                    }
                    Fields::Unit => quote! {
                        Self::#ident => {
                            #name.serialize_to(serializer)?;
                        }
                    },
                };
                match_statements.push(match_statement);
            }
            quote! {
                match self {
                    #(#match_statements)*
                }
            }
        }
        _ => panic!("only struct and enum are supported"),
    };

    quote! {
        impl #impl_generics derse::Serialize for #struct_type #ty_generics #where_clause {
            fn serialize_to<Serializer: derse::Serializer>(&self, serializer: &mut Serializer) -> derse::Result<()> {
                let start = serializer.len();
                #statements
                let len = serializer.len() - start;
                derse::VarInt64(len as u64).serialize_to(serializer)
            }
        }
    }.into()
}

#[proc_macro_derive(deserialize)]
pub fn derse_deserialize_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut generics = ast.generics.clone();
    let (impl_generics, lifetime) = if let Some(lifetime) = ast.generics.lifetimes().next().cloned()
    {
        (impl_generics, quote! { #lifetime })
    } else {
        let lifetime = Lifetime::new("'derse", proc_macro2::Span::call_site());
        let lifetime_param = LifetimeParam::new(lifetime.clone());
        let generic_param = syn::GenericParam::Lifetime(lifetime_param);
        generics.params.insert(0, generic_param);
        let (impl_generics, _, _) = generics.split_for_impl();
        (impl_generics, quote! { #lifetime })
    };

    let struct_type = &ast.ident;
    let struct_name = struct_type.to_string();
    let deserialize_statements = match ast.data {
        Data::Struct(DataStruct { fields, .. }) => {
            let statements = fields
                .iter()
                .map(|f| {
                    let statement = quote! {
                        if buf.is_empty() {
                            Default::default()
                        } else {
                            derse::Deserialize::deserialize_from(&mut buf)?
                        }
                    };
                    f.ident
                        .as_ref()
                        .map_or(statement.clone(), |ident| quote! {#ident: #statement})
                })
                .collect::<Vec<_>>();
            match fields {
                Fields::Named(_) => quote! { let result = Self { #(#statements, )* }; },
                Fields::Unnamed(_) => quote! { let result = Self ( #(#statements, )* ); },
                Fields::Unit => quote! { let result = Self; },
            }
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let mut match_statements = Vec::new();
            for variant in variants {
                let ident = &variant.ident;
                let variant_name = ident.to_string();
                let statements = variant
                    .fields
                    .iter()
                    .map(|f| {
                        let statement = quote! {
                            if buf.is_empty() {
                                Default::default()
                            } else {
                                derse::Deserialize::deserialize_from(&mut buf)?
                            }
                        };
                        f.ident
                            .as_ref()
                            .map_or(statement.clone(), |ident| quote! {#ident: #statement})
                    })
                    .collect::<Vec<_>>();
                let match_statement = match variant.fields {
                    Fields::Named(_) => {
                        quote! { #variant_name => Self::#ident { #(#statements, )* }, }
                    }
                    Fields::Unnamed(_) => {
                        quote! { #variant_name => Self::#ident ( #(#statements, )* ), }
                    }
                    Fields::Unit => quote! { #variant_name => Self::#ident, },
                };
                match_statements.push(match_statement);
            }
            quote! {
                let ty = <&str>::deserialize_from(&mut buf)?;
                let result = match ty {
                    #(#match_statements)*
                    _ => return Err(derse::Error::InvalidType(format!("{}::{}", #struct_name, ty))),
                };
            }
        }
        _ => panic!("only struct and enum are supported"),
    };

    quote! {
        impl #impl_generics #struct_type #ty_generics #where_clause {
            pub fn deserialize_and_split<Deserializer: derse::Deserializer<#lifetime>>(mut b: Deserializer) -> derse::Result<(Self, Deserializer)>
            where
                Self: Sized,
            {
                use derse::Deserialize;
                let mut buf = &mut b;
                let len = derse::VarInt64::deserialize_from(buf)?.0 as usize;
                let mut buf = buf.advance(len)?;
                #deserialize_statements
                Ok((result, buf))
            }
        }

        impl #impl_generics derse::Deserialize<#lifetime> for #struct_type #ty_generics #where_clause {
            fn deserialize_from<Deserializer: derse::Deserializer<#lifetime>>(buf: &mut Deserializer) -> derse::Result<Self>
            where
                Self: Sized,
            {
                let len = derse::VarInt64::deserialize_from(buf)?.0 as usize;
                let mut buf = buf.advance(len)?;
                #deserialize_statements
                Ok(result)
            }
        }
    }.into()
}
