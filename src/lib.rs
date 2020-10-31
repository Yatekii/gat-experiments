extern crate proc_macro;
use std::{iter::FromIterator, ops::Range};

use heck::{CamelCase, SnekCase};
use itertools::Itertools;
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
    name: Option<Ident>,
    type_name: Path,
}

#[derive(Debug)]
struct Characteristic {
    attributes: Range<usize>,
    descriptors: Range<usize>,
    name: Option<Ident>,
    type_name: Path,
}

#[derive(Debug)]
struct Descriptor {
    attributes: Range<usize>,
    name: Option<Ident>,
    type_name: Path,
}

#[derive(Debug)]
struct Attribute {
    data: String,
    size: usize,
    name: Option<Ident>,
    type_name: Path,
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
                name: input.name.clone(),
                type_name: input.type_name.clone(),
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
                name: input.name.clone(),
                type_name: input.type_name.clone(),
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
                name: input.name.clone(),
                type_name: input.type_name.clone(),
            });
        }
        Kind::Attribute => {
            let attribute = Attribute {
                data: input.type_name.get_ident().unwrap().to_string(),
                size: input.size.base10_parse().unwrap(), // TODO: Get rid of unwrap.
                name: input.name.clone(),
                type_name: input.type_name.clone(),
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

    let attribute_count = server.attributes.len();
    let mut store_size = 0;

    let attributes = server
        .attributes
        .iter()
        .map(|a| {
            let start = store_size;
            let size = a.size;
            store_size += a.size;
            quote! {
                Attribute {
                    att_type: 0,
                    handle: 0,
                    value: unsafe { core::mem::transmute::<&'static u8, &'static [u8; #size]>(&DATA_STORE[#start]) }
                }
            }
        })
        .collect::<Vec<_>>();

    let service_count = server.services.len();

    let services = server
        .services
        .iter()
        .map(|s| {
            let a_start = s.attributes.start;
            let a_end = s.attributes.end;
            let c_start = s.characteristics.start;
            let c_end = s.characteristics.end;
            quote! {
                Service {
                    // attributes: &ATTRIBUTES[#a_start..#a_end],
                    // characteristics: &CHARACTERISTICS[#c_start..#c_end]
                    attributes: &[],
                    characteristics: &[]
                }
            }
        })
        .collect::<Vec<_>>();

    let characteristic_count = server.characteristics.len();

    let characteristics = server
        .characteristics
        .iter()
        .map(|s| {
            let a_start = s.attributes.start;
            let a_end = s.attributes.end;
            let c_start = s.descriptors.start;
            let c_end = s.descriptors.end;
            quote! {
                Characteristic {
                    // attributes: &ATTRIBUTES[#a_start..#a_end],
                    // descriptors: &DESCRIPTORS[#c_start..#c_end]
                    attributes: &[],
                    descriptors: &[]
                }
            }
        })
        .collect::<Vec<_>>();

    let descriptor_count = server.descriptors.len();

    let descriptors = server
        .descriptors
        .iter()
        .map(|s| {
            let a_start = s.attributes.start;
            let a_end = s.attributes.end;
            quote! {
                Descriptor {
                    // attributes: &ATTRIBUTES[#a_start..#a_end],
                    attributes: &[],
                }
            }
        })
        .collect::<Vec<_>>();

    let (service_getters, service_types) = server
        .services
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let fn_name = s
                .name
                .clone()
                .or_else(|| {
                    s.type_name
                        .get_ident()
                        .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                })
                .unwrap();
            let type_name = s.type_name.clone();
            let handle_type_name = &mut s.type_name.clone();
            let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
            *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());

            let (cfn_name, chandle_name) = server.characteristics[s.characteristics.clone()].iter().map(|c| {
                let fn_name = c
                    .name
                    .clone()
                    .or_else(|| {
                        c.type_name
                            .get_ident()
                            .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                    })
                    .unwrap();
                let mut handle_type_name = c.type_name.clone();
                let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
                *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());
                (fn_name, handle_type_name)
            }).unzip::<_, _, Vec<_>, Vec<_>>();

            (quote! {
                pub fn #fn_name(&mut self) -> #handle_type_name {
                    #handle_type_name {
                        pd: core::marker::PhantomData {}
                    }
                }
            },
            quote! {
                pub struct #handle_type_name<'a> {
                    pd: core::marker::PhantomData<&'a mut ()>
                }

                impl core::ops::Deref for #handle_type_name<'_> {
                    type Target = #type_name;

                    fn deref(&self) -> &Self::Target {
                        unsafe { core::mem::transmute(&SERVICES[#i]) }
                    }
                }

                impl core::ops::DerefMut for #handle_type_name<'_> {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        unsafe {
                            &mut *(core::mem::transmute::<&Service, &#type_name>(&SERVICES[0usize])
                            as *const #type_name as *mut #type_name)
                        }
                    }
                }

                impl #handle_type_name<'_> {
                    #(
                        pub fn #cfn_name(&mut self) -> #chandle_name {
                            #chandle_name {
                                pd: core::marker::PhantomData {}
                            }
                        }
                    )*
                }
            })
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let characteristic_types = server
        .characteristics
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let fn_name = s
                .name
                .clone()
                .or_else(|| {
                    s.type_name
                        .get_ident()
                        .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                })
                .unwrap();
            let type_name = s.type_name.clone();
            let handle_type_name = &mut s.type_name.clone();
            let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
            *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());

            let (cfn_name, chandle_name) = server.descriptors[s.descriptors.clone()].iter().map(|c| {
                let fn_name = c
                    .name
                    .clone()
                    .or_else(|| {
                        c.type_name
                            .get_ident()
                            .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                    })
                    .unwrap();
                let mut handle_type_name = c.type_name.clone();
                let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
                *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());
                (fn_name, handle_type_name)
            }).unzip::<_, _, Vec<_>, Vec<_>>();

            quote! {
                pub struct #handle_type_name<'a> {
                    pd: core::marker::PhantomData<&'a mut ()>
                }

                impl core::ops::Deref for #handle_type_name<'_> {
                    type Target = #type_name;

                    fn deref(&self) -> &Self::Target {
                        unsafe { core::mem::transmute(&CHARACTERISTICS[#i]) }
                    }
                }

                impl core::ops::DerefMut for #handle_type_name<'_> {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        unsafe {
                            &mut *(core::mem::transmute::<&Characteristic, &#type_name>(&CHARACTERISTICS[0usize])
                            as *const #type_name as *mut #type_name)
                        }
                    }
                }

                impl #handle_type_name<'_> {
                    #(
                        pub fn #cfn_name(&mut self) -> #chandle_name {
                            #chandle_name {
                                pd: core::marker::PhantomData {}
                            }
                        }
                    )*
                }
            }
        })
        .collect::<Vec<_>>();

    let descriptor_types = server
        .descriptors
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let fn_name = s
                .name
                .clone()
                .or_else(|| {
                    s.type_name
                        .get_ident()
                        .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                })
                .unwrap();
            let type_name = s.type_name.clone();
            let handle_type_name = &mut s.type_name.clone();
            let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
            *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());

            let (cfn_name, chandle_name) = server.attributes[s.attributes.clone()].iter().map(|c| {
                let fn_name = c
                    .name
                    .clone()
                    .or_else(|| {
                        c.type_name
                            .get_ident()
                            .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                    })
                    .unwrap();
                let mut handle_type_name = c.type_name.clone();
                let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
                *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());
                (fn_name, handle_type_name)
            }).unzip::<_, _, Vec<_>, Vec<_>>();

            quote! {
                pub struct #handle_type_name<'a> {
                    pd: core::marker::PhantomData<&'a mut ()>
                }

                impl core::ops::Deref for #handle_type_name<'_> {
                    type Target = #type_name;

                    fn deref(&self) -> &Self::Target {
                        unsafe { core::mem::transmute(&DESCRIPTORS[#i]) }
                    }
                }

                impl core::ops::DerefMut for #handle_type_name<'_> {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        unsafe {
                            &mut *(core::mem::transmute::<&Descriptor, &#type_name>(&DESCRIPTORS[0usize])
                            as *const #type_name as *mut #type_name)
                        }
                    }
                }

                impl #handle_type_name<'_> {
                    #(
                        pub fn #cfn_name(&mut self) -> #chandle_name {
                            #chandle_name {
                                inner: unsafe { &mut *(&ATTRIBUTES[0usize] as *const Attribute as *mut Attribute) }
                            }
                        }
                    )*
                }
            }
        })
        .collect::<Vec<_>>();

    let attribute_types = server
        .attributes
        .iter()
        .unique_by(|s| s.type_name.clone())
        .map(|s| {
            let fn_name = s
                .name
                .clone()
                .or_else(|| {
                    s.type_name
                        .get_ident()
                        .map(|i| Ident::new(&i.to_string().to_snek_case(), i.span()))
                })
                .unwrap();
            let type_name = s.type_name.clone();
            let handle_type_name = &mut s.type_name.clone();
            let ident = &mut handle_type_name.segments.last_mut().unwrap().ident;
            *ident = Ident::new(&(ident.to_string() + "Handle"), ident.span());

            quote! {
                pub struct #handle_type_name<'a> {
                    inner: &'a mut Attribute,
                }

                impl core::ops::Deref for #handle_type_name<'_> {
                    type Target = #type_name;

                    fn deref(&self) -> &Self::Target {
                        unsafe { core::mem::transmute(&self.inner) }
                    }
                }

                impl core::ops::DerefMut for #handle_type_name<'_> {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        unsafe {
                            &mut *(core::mem::transmute::<&Attribute, &#type_name>(&self.inner)
                            as *const #type_name as *mut #type_name)
                        }
                    }
                }

                impl #handle_type_name<'_> {
                    pub fn get(&self) -> &[u8] {
                        &self.inner.value
                    }

                    pub fn set(&mut self, value: &[u8]) {
                        unsafe { self.inner.value.copy_from_slice(value); }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    (quote! {
        mod gatt_server {
            use super::*;
            static DATA_STORE: [u8; #store_size] = [0; #store_size];
            static ATTRIBUTES: [Attribute; #attribute_count] = [#(#attributes,)*];
            static SERVICES: [Service; #service_count] = [#(#services,)*];
            static CHARACTERISTICS: [Characteristic; #characteristic_count] = [#(#characteristics,)*];
            static DESCRIPTORS: [Descriptor; #descriptor_count] = [#(#descriptors,)*];
            
            static mut GAT_SERVER_TAKEN: bool = false;

            pub struct GattServer {}

            impl GattServer {
                pub fn take() -> Option<Self> {
                    // TODO:
                    // cortex_m::interrupt::free(|_| {
                        if unsafe { GAT_SERVER_TAKEN } {
                            None
                        } else {
                            Some(GattServer {})
                        }
                    // })
                }

                #(#service_getters)*
            }
            
            #(#service_types)*

            #(#characteristic_types)*

            #(#descriptor_types)*

            #(#attribute_types)*
        }
    })
    .into()
}
