
extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
extern crate quote;
extern crate convert_case;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::{parse_macro_input, parse::Parse, punctuated::{Punctuated}, Token, braced, Type, Expr};
use quote::quote;
use convert_case::{Case, Casing};

trait Gen {
    fn gen(&self) -> syn::Result<TokenStream2>;
    fn pregen(&self) -> syn::Result<TokenStream2>;
}

enum Deserialize {
    Default,
    With(Ident),
    Skip,
}

enum Key {
    Field(Ident, Type, bool, Deserialize, Option<Expr>),
    Sub(Ident, SubKeys, bool),
}

impl Key {
    pub fn default_value_fun(&self) -> Ident {
        match self {
            Key::Field(name, _, _, _, _) => Ident::new(
                &format!("default_for_{}", name.to_string()),
                name.span()
            ),
            
            Key::Sub(name, _, _) => Ident::new(
                &format!("default_for_{}", name.to_string()),
                name.span()
            ),
        }
    }

    pub fn name(&self) -> &Ident {
        match self {
            Key::Field(name, _, _, _, _) => name,
            Key::Sub(name, _, _) => name,
        }
    }

    pub fn type_name(&self) -> Option<Ident> {
        match self {
            Key::Field(_, _, _, _, _) => None,
            Key::Sub(name, _, _) =>
                Some(Ident::new(
                    &format!("{}Config", name.to_string().to_case(Case::Pascal)),
                    name.span()
                ))
        }
    }

    pub fn has_default_value(&self) -> bool {
        match self {
            Key::Field(_, _, opt, _, fun) => fun.is_some() || *opt,
            Key::Sub(_, sub, _) => {
                for key in &sub.keys {
                    if !key.has_default_value() {
                        return false;
                    }
                }
                true
            },
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            Key::Field(_, _, opt, _, _) => *opt,
            Key::Sub(_, _, opt) => *opt,
        }
    }
}

impl Gen for Key {
    fn gen(&self) -> syn::Result<TokenStream2> {
        match self {
            Key::Field(name, type_, optional, deser, default) => {
                let mut attrs = TokenStream2::new();

                attrs.extend(match deser {
                    Deserialize::Default => TokenStream2::new(),
                    Deserialize::Skip => quote!{skip},
                    Deserialize::With(w) => {
                        let deser_name = w.to_string();
                        quote!{deserialize_with = #deser_name}
                    },
                });

                if let Some(_) = default {
                    // Add comma if some attrs already exis
                    if !attrs.is_empty() {
                        attrs.extend(quote!{,})
                    }

                    let fun = self.default_value_fun().to_string();
                    attrs.extend(quote!{default = #fun})
                }

                if !attrs.is_empty() {
                    attrs = quote!{ #[serde(#attrs)] }
                }

                if *optional {
                    Ok(quote! {
                        #attrs
                        pub #name: Option<#type_>,
                    })
                }
                else {
                    Ok(quote! {
                        #attrs
                        pub #name: #type_,
                    })
                }
            },

            Key::Sub(name, _, optional) => {
                let type_ = &self.type_name();
                if *optional {
                    Ok(quote! {
                        pub #name: Option<#type_>,
                    })
                }
                else {
                    if self.has_default_value() {
                        Ok(quote! {
                            #[serde(default)]
                            pub #name: #type_,
                        })
                    }
                    else {
                        Ok(quote! {
                            pub #name: #type_,
                        })
                    }
                }
            },
        }
    }

    fn pregen(&self) -> syn::Result<TokenStream2> {
        match self {
            Key::Field(_, type_, _, _, default) => {
                if let Some(fun) = default {
                    let name = self.default_value_fun();
                    Ok(quote! {
                        fn #name () -> #type_ {
                            #fun
                        }
                    })
                }
                else {
                    Ok(TokenStream2::new())
                }
            },

            Key::Sub(_, sub, _) => {
                let name = &self.type_name();
                let pregen = sub.pregen()?;
                let keys = sub.gen()?;

                // impl Default
                let default = if self.has_default_value() {
                    let mut default_stream = TokenStream2::new();
                    for key in &sub.keys {
                        let key_name = key.name();
                        if key.is_optional() {
                            default_stream.extend(quote! {
                                #key_name: None,
                            });
                        }
                        else {
                            let key_fun = key.default_value_fun();
                            default_stream.extend(quote! {
                                #key_name: #key_fun (),
                            });
                        }
                    }
                    Some(quote! {
                        impl Default for #name {
                            fn default() -> Self {
                                Self {
                                    #default_stream
                                }
                            }
                        }
                    })
                } else {
                    None
                };

                Ok(quote! {
                    #pregen

                    #[derive(Deserialize)]
                    #[serde(rename_all = "kebab-case")]
                    pub struct #name {
                        #keys
                    }

                    #default
                })
            },
        }
    }
}

impl Parse for Key {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // If first token is 'let' add #[serde(skip)]
        let deser_skip = input.parse::<Token![let]>().is_ok();

        // Parse name for field
        let name = input.parse::<Ident>()?;

        let optional = input.parse::<Token![?]>().is_ok();

        // If there's a colon and a type, it's a direct field
        if input.parse::<Token![:]>().is_ok() {
            Ok(Key::Field(
                // Field name
                name,
                // Type
                input.parse()?,
                // Optional
                optional,
                // Deserialization function
                if deser_skip {
                    Deserialize::Skip
                } else {
                    if input.parse::<Token![as]>().is_ok() {
                        Deserialize::With(input.parse::<Ident>()?)
                    } else {
                        Deserialize::Default
                    }
                },
                // Default value
                if input.parse::<Token![=]>().is_ok() {
                    Some(input.parse()?)
                } else {
                    None
                },
            ))
        }
        // Otherwise it's a sub struct
        else {
            let content;
            braced!(content in input);
            Ok(Key::Sub(name, content.parse()?, optional))
        }
    }
}

struct SubKeys {
    keys: Punctuated<Key, Token![,]>,
}

impl Gen for SubKeys {
    fn gen(&self) -> syn::Result<TokenStream2> {
        let mut stream = TokenStream2::new();
        for key in &self.keys {
            stream.extend(key.gen()?);
        }
        Ok(stream)
    }

    fn pregen(&self) -> syn::Result<TokenStream2> {
        let mut stream = TokenStream2::new();
        for key in &self.keys {
            stream.extend(key.pregen()?);
        }
        Ok(stream)
    }
}

impl Parse for SubKeys {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            keys: Punctuated::parse_terminated(input)?,
        })
    }
}

struct ConfigDecl {
    name: Ident,
    keys: SubKeys,
}

impl Gen for ConfigDecl {
    fn gen(&self) -> syn::Result<TokenStream2> {
        // Struct defs
        let pregen = self.keys.pregen()?;
        let gen = self.keys.gen()?;

        let name = &self.name;
        Ok(quote! {
            #pregen

            #[derive(Deserialize)]
            #[serde(rename_all = "kebab-case")]
            pub struct #name {
                #gen
            }
        })
    }

    fn pregen(&self) -> syn::Result<TokenStream2> {
        unreachable!()
    }
}

impl Parse for ConfigDecl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![struct]>()?;
        let name = input.parse()?;
        let content;
        braced!(content in input);
        Ok(Self {
            name,
            keys: content.parse()?,
        })
    }
}

struct MultiConfigDecl {
    configs: Vec<ConfigDecl>,
}

impl Gen for MultiConfigDecl {
    fn gen(&self) -> syn::Result<TokenStream2> {
        let mut stream = TokenStream2::new();
        for config in &self.configs {
            stream.extend(config.gen()?);
        }
        Ok(stream)
    }

    fn pregen(&self) -> syn::Result<TokenStream2> {
        unreachable!()
    }
}

impl Parse for MultiConfigDecl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut configs = vec![];
        while let Ok(i) = input.parse() {
            configs.push(i);
        }
        Ok(Self {
            configs
        })
    }
}

#[proc_macro]
pub fn decl_config(input: TokenStream) -> TokenStream {
    match parse_macro_input!(input as MultiConfigDecl).gen() {
        Ok(s) => s.into(),
        Err(e) => TokenStream::from(e.to_compile_error()),
    }
}
