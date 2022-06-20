use crate::{Protocol, ValueType};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use std::iter;

pub struct CodeBuilder {
    interface_names: Vec<String>,
    interface_versions: Vec<u32>,
    dispatch_functions_paths: Vec<Vec<TokenStream>>,
    protocols: Vec<TokenStream>,
    demarshaller_signature: TokenStream,
}

impl Default for CodeBuilder {
    fn default() -> Self {
        let demarshaller_signature = quote! {
            (object: &mut Interface, args: &[u32], state: &mut DispatchState) -> Result<(), MessageError>
        };

        Self {
            interface_names: vec![],
            interface_versions: vec![],
            dispatch_functions_paths: vec![],
            protocols: vec![],
            demarshaller_signature,
        }
    }
}

impl CodeBuilder {
    pub fn build(self) -> TokenStream {
        let Self {
            interface_names,
            interface_versions,
            dispatch_functions_paths,
            protocols,
            demarshaller_signature,
            ..
        } = self;

        assert_eq!(interface_names.len(), interface_versions.len());
        let interface_count = interface_names.len();

        let max_request_count = dispatch_functions_paths
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0);
        let dispatch_entries = dispatch_functions_paths.into_iter().map(|paths| {
            let funcs = paths
                .into_iter()
                .map(|path| quote! { Some(#path) })
                .chain(iter::repeat(quote! { None }))
                .take(max_request_count);

            quote! {
                [#(#funcs),*]
            }
        });

        let interface_enum_variants = interface_names.iter().map(|name| {
            let name = format_ident!("{}", name.to_case(Case::Pascal));
            quote! { #name(protocol::#name) }
        });

        quote! {
            use crate::{
                gateway::{
                    message::MessageError,
                    registry::ObjectId,
                },
                protocol::{self, DispatchState},
            };
            use fixed::types::I24F8;
            use bytemuck::cast_slice;
            use std::os::unix::io::RawFd;

            pub type RequestDemarshaller = fn #demarshaller_signature;
            type DispatchEntry = [Option<RequestDemarshaller>; #max_request_count];

            pub static INTERFACE_NAMES: [&str; #interface_count] = [#(#interface_names),*];
            pub static INTERFACE_VERSIONS: [u32; #interface_count] = [#(#interface_versions),*];
            pub static INTERFACE_DISPATCH_TABLE: [DispatchEntry; #interface_count] = [#(#dispatch_entries),*];

            #(#protocols)*

            pub enum Interface {
                #(#interface_enum_variants),*
            }
        }
    }

    pub fn add_protocol(&mut self, protocol: Protocol) {
        let interfaces = protocol.interfaces.iter().map(|interface| {
            let interface_mod = format_ident!("{}", interface.name);
            let interface_struct = format_ident!("{}", interface.name.to_case(Case::Pascal));

            let enums = interface.enums.iter().map(|enum_| {
                let name = format_ident!("{}", enum_.name.to_case(Case::Pascal));

                fn convert_variant_name(name: &str) -> Ident {
                    let name = name.to_case(Case::Pascal);
                    match name.parse::<u32>() {
                        Ok(_) => format_ident!("U{}", name),
                        Err(_) => format_ident!("{}", name),
                    }
                }
                let entries = enum_.entries.iter().map(|(name, value)| {
                    let name = convert_variant_name(name);
                    quote! { #name = #value }
                });
                let match_entries = enum_.entries.iter().map(|(name, value)| {
                    let name = convert_variant_name(name);
                    quote! { #value => Ok(Self::#name) }
                });

                quote! {
                    #[repr(u32)]
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
                    pub enum #name {
                        #(#entries),*
                    }

                    impl TryFrom<u32> for #name {
                        type Error = MessageError;

                        fn try_from(v: u32) -> Result<Self, MessageError> {
                            match v {
                                #(#match_entries),*,
                                _ => Err(MessageError::BadFormat),
                            }
                        }
                    }
                }
            });

            let request_dispatches = interface.requests.iter().map(|request| {
                let extract_args = request.args.iter().map(|arg| {
                    let arg_size;
                    let cur_chunk = quote! {
                        (*args.get(__a).ok_or(MessageError::BadFormat)?)
                    };
                    let name = format_ident!("{}", &arg.name);
                    let extract = match &arg.value_type {
                        ValueType::I32 => {
                            arg_size = quote! { 1 };
                            quote! { #cur_chunk as i32 }
                        }
                        ValueType::U32 => {
                            arg_size = quote! { 1 };
                            quote! { #cur_chunk }
                        }
                        ValueType::Enum { interface, name } => {
                            arg_size = quote! { 1 };
                            let name = format_ident!("{}", name.to_case(Case::Pascal));
                            let enum_ty = match interface.as_ref().map(|i| format_ident!("{}", i)) {
                                Some(interface) => quote! { #interface::#name },
                                None => quote! { #name },
                            };
                            quote! { #enum_ty::try_from(#cur_chunk)? }
                        }
                        ValueType::Fixed => {
                            arg_size = quote! { 1 };
                            quote! { I24F8::from_ne_bytes(#cur_chunk.to_ne_bytes()) }
                        }
                        ValueType::ObjectId {
                            optional,
                            ..
                        } => {
                            arg_size = quote! { 1 };
                            let option = quote! { ObjectId::new(#cur_chunk) };

                            if *optional {
                                option
                            } else {
                                quote! { #option.ok_or(MessageError::BadFormat)? }
                            }
                        }
                        ValueType::String { optional } => {
                            arg_size = quote! { (#cur_chunk as usize + 3) / 4 };
                            let create_option = quote! {
                                #cur_chunk
                                    .checked_sub(1)
                                    .map(|len| {
                                        args
                                            .get(__a + 1..)
                                            .and_then(|words| {
                                                cast_slice::<_, u8>(words)
                                                    .get(..len as usize)
                                                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                                            })
                                            .ok_or(MessageError::BadFormat)
                                    })
                                    .transpose()?
                            };
                            if *optional {
                                create_option
                            } else {
                                quote! {
                                    #create_option.ok_or(MessageError::BadFormat)?
                                }
                            }
                        }
                        ValueType::Array { optional } => {
                            arg_size = quote! { (#cur_chunk as usize + 3) / 4 };
                            if *optional {
                                todo!("don't know how optional arrays are encoded")
                            } else {
                                quote! {
                                    args
                                        .get(__a + 1..)
                                        .and_then(|words| {
                                            cast_slice::<_, u8>(words)
                                                .get(..#cur_chunk as usize)
                                        })
                                        .ok_or(MessageError::BadFormat)?
                                }
                            }
                        }
                        ValueType::Fd => {
                            arg_size = quote! { 0 };
                            quote! {
                                state.fds.pop().ok_or(MessageError::BadFormat)?
                            }
                        }
                    };

                    quote! {
                        let #name = #extract;
                        __a += #arg_size;
                    }
                });

                let fn_name = format_ident!("handle_{}", &request.name);
                let demarshaller_signature = &self.demarshaller_signature;
                let args = request.args.iter().map(|arg| format_ident!("{}", arg.name));

                quote! {
                    pub fn #fn_name #demarshaller_signature {
                        let object = protocol::#interface_struct::downcast(object)
                            .expect("Demarshaller called with invalid object");

                        let mut __a = 0;
                        #(#extract_args)*
                        if __a == args.len() {
                            object.#fn_name(state, #(#args),*)
                        } else {
                            Err(MessageError::BadFormat)
                        }
                    }
                }
            });

            let events = interface.events.iter().map(|event| {
                let fn_name = format_ident!("emit_{}", &event.name);
                let args = event.args.iter().map(|arg| {
                    let name = format_ident!("{}", &arg.name);
                    let value_type = match &arg.value_type {
                        ValueType::I32 => quote! { i32 },
                        ValueType::U32 => quote! { u32 },
                        ValueType::Enum { interface, name } => {
                            let name = format_ident!("{}", name.to_case(Case::Pascal));
                            if let Some(interface) = interface.as_ref().map(|i| format_ident!("{}", i))
                            {
                                quote! {
                                    #interface::#name
                                }
                            } else {
                                quote! {
                                    #name
                                }
                            }
                        }
                        ValueType::Fixed => quote! { I24F8 },
                        ValueType::ObjectId {
                            interface,
                            optional,
                        } => {
                            let id_type = if let Some(interface) = interface.as_ref() {
                                let interface = format_ident!("{}", interface.to_case(Case::Pascal));
                                quote! { ObjectId<protocol::#interface> }
                            } else {
                                quote! { ObjectId<Interface> }
                            };

                            if *optional {
                                quote! { Option<#id_type> }
                            } else {
                                quote! { #id_type }
                            }
                        }
                        ValueType::String { optional } => {
                            if *optional {
                                quote! { Option<&str> }
                            } else {
                                quote! { &str }
                            }
                        }
                        ValueType::Array { optional } => {
                            if *optional {
                                quote! { Option<&[u8]> }
                            } else {
                                quote! { &[u8] }
                            }
                        }
                        ValueType::Fd => quote! { RawFd },
                    };

                    quote! { #name: #value_type }
                });

                quote! {
                    pub fn #fn_name(self_id: ObjectId<protocol::#interface_struct>, #(#args),*) {}
                }
            });

            quote! {
                pub mod #interface_mod {
                    use super::*;

                    #(#enums)*
                    #(#request_dispatches)*
                    #(#events)*
                }

                impl protocol::#interface_struct {
                    fn downcast(object: &mut Interface) -> Option<&mut Self> {
                        match object {
                            Interface::#interface_struct(v) => Some(v),
                            _ => None,
                        }
                    }
                }
            }
        });

        let protocol_tokens = quote! {
            #(#interfaces)*
        };

        for interface in protocol.interfaces {
            let interface_mod = format_ident!("{}", interface.name);
            self.interface_names.push(interface.name);
            self.interface_versions.push(interface.version);
            let paths = interface
                .requests
                .into_iter()
                .map(|r| {
                    let fn_name = format_ident!("handle_{}", r.name);
                    quote! { #interface_mod::#fn_name }
                })
                .collect();
            self.dispatch_functions_paths.push(paths);
        }

        self.protocols.push(protocol_tokens);
    }
}
