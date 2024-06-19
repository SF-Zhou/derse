extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Lifetime, LifetimeParam};

#[proc_macro_derive(Derse)]
pub fn derse_derive(input: TokenStream) -> TokenStream {
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

    let name = &ast.ident;
    let mut serialize_fields = Vec::new();
    let mut deserialize_fields = Vec::new();

    let deserialize_statement = match ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            for f in fields.named {
                let field_name = &f.ident;
                serialize_fields.push(quote! {
                    self.#field_name.serialize_to(serializer)?;
                });
                deserialize_fields.push(quote! {
                    #field_name: if buf.is_empty() {
                        Default::default()
                    } else {
                        ::derse::Serialization::deserialize_from(&mut buf)?
                    },
                });
            }
            quote! {
                Self {
                    #(#deserialize_fields)*
                }
            }
        }
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => {
            for (i, _) in fields.unnamed.iter().enumerate() {
                let index = syn::Index::from(i);
                serialize_fields.push(quote! {
                    self.#index.serialize_to(serializer)?;
                });
                deserialize_fields.push(quote! {
                    if buf.is_empty() {
                        Default::default()
                    } else {
                        ::derse::Serialization::deserialize_from(&mut buf)?
                    },
                });
            }
            quote! {
                Self (
                    #(#deserialize_fields)*
                )
            }
        }
        _ => panic!("only struct is supported"),
    };

    serialize_fields.reverse();

    let gen = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn deserialize_and_split<Deserializer: ::derse::Deserializer<#lifetime>>(mut b: Deserializer) -> ::derse::Result<(Self, Deserializer)>
            where
                Self: Sized,
            {
                let mut buf = &mut b;
                let len = ::derse::VarInt64::deserialize_from(buf)?.0 as usize;
                let mut buf = buf.advance(len)?;
                Ok((#deserialize_statement, buf))
            }
        }

        impl #impl_generics ::derse::Serialization<#lifetime> for #name #ty_generics #where_clause {
            fn serialize_to<Serializer: ::derse::Serializer>(&self, serializer: &mut Serializer) -> ::derse::Result<()> {
                let start = serializer.len();
                #(#serialize_fields)*
                let len = serializer.len() - start;
                ::derse::VarInt64(len as u64).serialize_to(serializer)
            }

            fn deserialize_from<Deserializer: ::derse::Deserializer<#lifetime>>(buf: &mut Deserializer) -> ::derse::Result<Self>
            where
                Self: Sized,
            {
                let len = ::derse::VarInt64::deserialize_from(buf)?.0 as usize;
                let mut buf = buf.advance(len)?;
                Ok(#deserialize_statement)
            }
        }
    };

    gen.into()
}
