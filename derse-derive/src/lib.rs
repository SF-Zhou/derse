extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(Derse)]
pub fn derse_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = if ast.generics.lifetimes().count() > 0 {
        let name = &ast.ident;
        quote! { #name::<'a> }
    } else {
        ast.ident.to_token_stream()
    };
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
                    self.#field_name.serialize_to(serializer);
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
                    self.#index.serialize_to(serializer);
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
        impl<'a> ::derse::Serialization<'a> for #name {
            fn serialize_to<T: ::derse::Serializer>(&self, serializer: &mut T) {
                let start = serializer.len();
                #(#serialize_fields)*
                let len = serializer.len() - start;
                ::derse::VarInt64(len as u64).serialize_to(serializer);
            }

            fn deserialize_from(buf: &mut &'a [u8]) -> ::derse::Result<Self>
            where
                Self: Sized,
            {
                let len = ::derse::VarInt64::deserialize_from(buf)?.0 as usize;
                if buf.len() < len {
                    return Err(::derse::Error::DataIsShort {
                        expect: len,
                        actual: buf.len(),
                    });
                }

                let current = &buf[..len];
                *buf = &buf[len..];

                let mut buf = current;
                Ok(#deserialize_statement)
            }
        }
    };

    gen.into()
}
