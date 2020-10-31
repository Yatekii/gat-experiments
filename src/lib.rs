extern crate proc_macro;
use std::{iter::FromIterator, ops::Range};

use heck::CamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::Brace,
    token::Paren,
    token::{self, Pub},
    Error, Expr, ExprLit, ExprPath, Field, Fields, FieldsUnnamed, Ident, Lit, LitInt, LitStr, Path,
    PathArguments, PathSegment, Token, Type, TypePath, VisPublic, Visibility,
};

#[derive(Debug)]
struct GattServerParsed {
    struct_likes: Vec<StructLike>,
}

#[derive(Debug)]
enum Kind {
    Service,
    Characteristic,
    Descriptor,
    Attribute,
}

mod kw {
    syn::custom_keyword!(service);
    syn::custom_keyword!(characteristic);
    syn::custom_keyword!(descriptor);
    syn::custom_keyword!(attribute);
}

impl Parse for Kind {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::service) {
            input.parse::<Ident>()?;
            Ok(Kind::Service)
        } else if input.peek(kw::characteristic) {
            input.parse::<Ident>()?;
            Ok(Kind::Characteristic)
        } else if input.peek(kw::descriptor) {
            input.parse::<Ident>()?;
            Ok(Kind::Descriptor)
        } else if input.peek(kw::attribute) {
            input.parse::<Ident>()?;
            Ok(Kind::Attribute)
        } else {
            Err(input.error("Expected a valid path segment"))
        }
    }
}

#[derive(Debug)]
struct StructLike {
    kind: Kind,
    name: Option<Ident>,
    type_name: Path,
    children: Vec<StructLike>,
    size: LitInt,
}

impl Parse for StructLike {
    fn parse(input: ParseStream) -> Result<Self> {
        let kind = input.parse()?;
        let name = if input.peek(Ident) {
            Some(input.parse()?)
        } else {
            None
        };
        input.parse::<Token![:]>()?;
        let type_name = input.parse()?;
        let mut children = vec![];
        let mut size = LitInt::new("0", Span::call_site());
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            match kind {
                Kind::Attribute => size = content.parse()?,
                _ => {
                    children = Punctuated::<StructLike, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect()
                }
            }
        }
        Ok(StructLike {
            kind,
            name,
            type_name,
            children,
            size,
        })
    }
}

impl Parse for GattServerParsed {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(GattServerParsed {
            struct_likes: Punctuated::<StructLike, Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect(),
        })
    }
}

#[derive(Debug)]
struct Service {
    attributes: Range<usize>,
    characteristics: Range<usize>,
}

#[derive(Debug)]
struct Characteristic {
    attributes: Range<usize>,
    descriptors: Range<usize>,
}

#[derive(Debug)]
struct Descriptor {
    attributes: Range<usize>,
}

#[derive(Debug)]
struct Attribute {
    data: String,
    size: usize,
}

#[derive(Debug)]
struct GattServer {
    services: Vec<Service>,
    characteristics: Vec<Characteristic>,
    descriptors: Vec<Descriptor>,
    attributes: Vec<Attribute>,
}

fn recurse_structs(server: &mut GattServer, input: &StructLike) {
    match input.kind {
        Kind::Service => {
            let mut characteristics = vec![];
            let mut attributes = vec![];
            for child in &input.children {
                match child.kind {
                    Kind::Characteristic => characteristics.push(child),
                    Kind::Attribute => attributes.push(child),
                    _ => (), // TODO: Error
                }
            }
            let cc = server.characteristics.len();
            for c in &characteristics {
                recurse_structs(server, &c);
            }
            let ac = server.attributes.len();
            for a in &attributes {
                recurse_structs(server, &a);
            }
            server.services.push(Service {
                attributes: ac..ac + attributes.len(),
                characteristics: cc..cc + characteristics.len(),
            });
        }
        Kind::Characteristic => {
            let mut descriptors = vec![];
            let mut attributes = vec![];
            for child in &input.children {
                match child.kind {
                    Kind::Descriptor => descriptors.push(child),
                    Kind::Attribute => attributes.push(child),
                    _ => (), // TODO: Error
                }
            }
            let dc = server.descriptors.len();
            for c in &descriptors {
                recurse_structs(server, &c);
            }
            let ac = server.attributes.len();
            for a in &attributes {
                recurse_structs(server, &a);
            }
            server.characteristics.push(Characteristic {
                attributes: ac..ac + attributes.len(),
                descriptors: dc..dc + descriptors.len(),
            });
        }
        Kind::Descriptor => {
            let mut attributes = vec![];
            for child in &input.children {
                match child.kind {
                    Kind::Attribute => attributes.push(child),
                    _ => (), // TODO: Error
                }
            }
            let ac = server.attributes.len();
            for a in &attributes {
                recurse_structs(server, &a);
            }
            server.descriptors.push(Descriptor {
                attributes: ac..ac + attributes.len(),
            });
        }
        Kind::Attribute => {
            let attribute = Attribute {
                data: input.type_name.get_ident().unwrap().to_string(),
                size: input.size.base10_parse().unwrap(), // TODO: Get rid of unwrap.
            };
            if input.children.len() > 0 {
                // TODO: Error
            }
            server.attributes.push(attribute);
        }
    }
}

#[proc_macro]
pub fn gatt_server(input: TokenStream) -> TokenStream {
    let server_parsed = parse_macro_input!(input as GattServerParsed);

    let mut server = GattServer {
        services: vec![],
        characteristics: vec![],
        descriptors: vec![],
        attributes: vec![],
    };

    for child in server_parsed.struct_likes {
        recurse_structs(&mut server, &child);
    }

    println!("{:#?}", server);

    let attribute_count = server.attributes.len();
    let mut store_size = 0;

    let attributes = server
        .attributes
        .iter()
        .map(|a| {
            let previous_size = store_size;
            store_size += a.size;
            quote! {
                Attribute {
                    att_type: 0,
                    handle: 0,
                    value: &DATA_STORE[#previous_size..#store_size],
                }
            }
        })
        .collect::<Vec<_>>();

    (quote! {
        static DATA_STORE: [u8; #store_size] = [0; #store_size];
        static ATTRIBUTES: [Attribute<'static>; #attribute_count] = [#(#attributes,)*];
    })
    .into()
}
