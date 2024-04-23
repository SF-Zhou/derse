extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(Derse)]
pub fn derse_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;
    let mut field_types = Vec::new();

    let fields_tokens = if let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = ast.data
    {
        fields
            .named
            .iter()
            .map(|f| {
                let field_name = &f.ident;
                let field_type = &f.ty;
                field_types.push(field_name.as_ref().unwrap().to_owned());
                quote! {
                    #field_name: if buf.is_empty() {
                        #field_type::default()
                    } else {
                        #field_type::deserialize_from(&mut buf)?
                    },
                }
            })
            .collect::<Vec<_>>()
    } else {
        panic!("only named fields supported")
    };

    field_types.reverse();

    let gen = quote! {
        impl<'a> ::derse::Serialization<'a> for #name {
            fn serialize_to<T: ::derse::Serializer>(&self, serializer: &mut T) {
                let start = serializer.len();
                #(self.#field_types.serialize_to(serializer);)*
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
                Ok(Self {
                    #(#fields_tokens)*
                })
            }
        }
    };

    gen.into()
}
