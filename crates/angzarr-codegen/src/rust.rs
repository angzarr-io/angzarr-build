//! The Rust emitter: a package's domain layer on annals, either complete
//! ([`package`]) or for marked-up components ([`markup_package`]).
//!
//! Every domain type is a plain Rust type with its wire conversions: into the
//! prost message infallibly, back out fallibly with annals' `WireError`
//! naming the field. Message fields are required (the wire's `Option` is
//! gone), proto3 `optional` stays an `Option`, and enums are closed: the
//! zero value (`*_UNSPECIFIED`) and any number without a variant are refused.
//! Enums convert by number, so nothing here depends on how prost names
//! variants.

use proc_macro2::{Span, TokenStream};
use prost_reflect::{EnumDescriptor, FieldDescriptor, Kind, MessageDescriptor};
use quote::{format_ident, quote};

use crate::model::{Component, Package, TIMESTAMP};

/// The runtime crate the generated code targets, as its Rust identifier:
/// every path into it, the macros crate's name (`<RUNTIME>_macros`) and the
/// prose naming it in generated docs are built from this one name.
pub(crate) const RUNTIME: &str = "annals";

/// The runtime crate's identifier.
pub(crate) fn runtime() -> syn::Ident {
    format_ident!("{RUNTIME}")
}

/// The runtime's markup macros crate's identifier.
pub(crate) fn runtime_macros() -> syn::Ident {
    format_ident!("{RUNTIME}_macros")
}

/// `::<runtime>::next`, the runtime API the generated code is written
/// against.
pub(crate) fn next() -> TokenStream {
    let runtime = runtime();
    quote!(::#runtime::next)
}

/// The generated source for one package. `wire` is the Rust path of its
/// prost types.
pub(crate) fn package(package: &Package, wire: &syn::Path) -> String {
    let mut items = TokenStream::new();
    for e in &package.enums {
        items.extend(enumeration(e));
    }
    for message in &package.messages {
        let anchor = package.components.iter().any(|c| &c.anchor == message);
        items.extend(structure(message, wire, anchor));
    }
    for component in &package.components {
        items.extend(component_items(component, wire));
    }
    let file: syn::File = syn::parse2(items).expect("the emitter writes valid Rust");
    prettyplease::unparse(&file)
}

/// The markup domain layer for one package: its domain types with their
/// wire conversions, and a `DomainMessage` impl for every input, event and
/// refusal of its components. No payload enums and no `State` impl: the
/// markup generates those.
pub(crate) fn markup_package(package: &Package, wire: &syn::Path) -> String {
    let mut items = TokenStream::new();
    for e in &package.enums {
        items.extend(enumeration(e));
    }
    for message in &package.messages {
        items.extend(structure(message, wire, false));
    }
    let mut messages: Vec<&MessageDescriptor> = Vec::new();
    for component in &package.components {
        let declared = component
            .inputs
            .iter()
            .flat_map(|i| std::iter::once(&i.message).chain(&i.refuses))
            .chain(&component.events);
        for message in declared {
            if !messages.contains(&message) {
                messages.push(message);
            }
        }
    }
    let next = next();
    for message in messages {
        let name = ident(message.name());
        items.extend(quote! {
            impl #next::markup::DomainMessage for #name {
                type Wire = #wire::#name;

                fn to_wire(&self) -> #wire::#name {
                    #wire::#name::from(self.clone())
                }

                fn from_wire(wire: #wire::#name) -> Result<Self, #next::WireError> {
                    #name::try_from(wire)
                }
            }
        });
    }
    let file: syn::File = syn::parse2(items).expect("the emitter writes valid Rust");
    prettyplease::unparse(&file)
}

fn enumeration(e: &EnumDescriptor) -> TokenStream {
    let name = ident(e.name());
    let prefix = format!("{}_", screaming(e.name()));
    let values: Vec<(syn::Ident, i32)> = e
        .values()
        .filter(|v| v.number() != 0)
        .map(|v| {
            let bare = v.name().strip_prefix(&prefix).unwrap_or(v.name());
            (ident(&camel(bare)), v.number())
        })
        .collect();
    let variants = values.iter().map(|(v, _)| v);
    let into = values.iter().map(|(v, n)| quote!(#name::#v => #n));
    let from = values.iter().map(|(v, n)| quote!(#n => Ok(#name::#v)));
    quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum #name { #(#variants),* }

        impl From<#name> for i32 {
            fn from(value: #name) -> i32 {
                match value { #(#into),* }
            }
        }

        impl TryFrom<i32> for #name {
            type Error = i32;

            fn try_from(value: i32) -> Result<Self, i32> {
                match value {
                    #(#from,)*
                    other => Err(other),
                }
            }
        }
    }
}

fn structure(message: &MessageDescriptor, wire: &syn::Path, anchor: bool) -> TokenStream {
    let next = next();
    let name = ident(message.name());
    let full = message.full_name();
    let fields: Vec<Field> = message.fields().map(|f| Field::of(&f, wire)).collect();
    let decls = fields.iter().map(|f| {
        let (n, ty) = (&f.name, f.shape.domain_type());
        quote!(pub #n: #ty)
    });
    let to = fields.iter().map(|f| {
        let n = &f.name;
        let value = f.shape.encoded(quote!(value.#n));
        quote!(#n: #value)
    });
    let from = fields.iter().map(|f| {
        let n = &f.name;
        let value = f.shape.decoded(quote!(value.#n), full, &f.proto_name);
        quote!(#n: #value)
    });
    let param = if fields.is_empty() {
        quote!(_value)
    } else {
        quote!(value)
    };
    let derives = if anchor {
        quote!(#[derive(Clone, Debug, Default, PartialEq)])
    } else {
        quote!(#[derive(Clone, Debug, PartialEq)])
    };
    quote! {
        #derives
        pub struct #name { #(#decls),* }

        impl From<#name> for #wire::#name {
            fn from(#param: #name) -> Self {
                #wire::#name { #(#to),* }
            }
        }

        impl TryFrom<#wire::#name> for #name {
            type Error = #next::WireError;

            fn try_from(#param: #wire::#name) -> Result<Self, Self::Error> {
                Ok(#name { #(#from),* })
            }
        }
    }
}

fn component_items(component: &Component, wire: &syn::Path) -> TokenStream {
    let next = next();
    let domain_const = format_ident!("{}_DOMAIN", screaming(&component.name));
    let domain = &component.domain;
    let inputs: Vec<MessageDescriptor> =
        component.inputs.iter().map(|i| i.message.clone()).collect();
    let commands = payload(&format_ident!("{}Command", component.name), &inputs, wire);
    let events = payload(
        &format_ident!("{}Event", component.name),
        &component.events,
        wire,
    );
    let state = ident(component.anchor.name());
    quote! {
        pub const #domain_const: &str = #domain;

        #commands
        #events

        impl #next::State for #state {
            type Snapshot = #wire::#state;

            fn to_snapshot(&self) -> #wire::#state {
                #wire::#state::from(self.clone())
            }

            fn from_snapshot(
                snapshot: #wire::#state,
            ) -> Result<Self, ::annals_diagnostic::Report> {
                #state::try_from(snapshot).map_err(|error| ::annals_diagnostic::Report::of(&error))
            }

            fn descriptor() -> #next::v1::StateDescriptor {
                #next::v1::StateDescriptor {
                    type_url: <#wire::#state as ::prost::Name>::type_url(),
                    options: Some(#next::v1::StateOptions {
                        conflict_resolutions: vec![
                            #next::v1::ConflictResolution::Strict as i32,
                            #next::v1::ConflictResolution::AggregateHandles as i32,
                        ],
                    }),
                }
            }
        }
    }
}

/// An enum of `messages`, one variant each, as annals' `Payload`.
fn payload(name: &syn::Ident, messages: &[MessageDescriptor], wire: &syn::Path) -> TokenStream {
    let next = next();
    let variants: Vec<syn::Ident> = messages.iter().map(|m| ident(m.name())).collect();
    let to_any = variants
        .iter()
        .map(|v| quote!(#name::#v(value) => #next::pack(&#wire::#v::from(value.clone()))));
    let from_any = variants.iter().map(|v| {
        quote! {
            if name == <#wire::#v as ::prost::Name>::full_name() {
                return #next::decode_as::<#wire::#v, #v>(any).map(#name::#v);
            }
        }
    });
    let type_urls = variants
        .iter()
        .map(|v| quote!(<#wire::#v as ::prost::Name>::type_url()));
    let froms = variants.iter().map(|v| {
        quote! {
            impl From<#v> for #name {
                fn from(value: #v) -> Self {
                    #name::#v(value)
                }
            }
        }
    });
    quote! {
        #[derive(Clone, Debug, PartialEq)]
        pub enum #name { #(#variants(#variants)),* }

        #(#froms)*

        impl #next::Payload for #name {
            fn to_any(&self) -> #next::Any {
                match self { #(#to_any),* }
            }

            fn from_any(
                any: &#next::Any,
            ) -> Result<Self, #next::PayloadError> {
                let name = #next::type_name(&any.type_url);
                #(#from_any)*
                Err(#next::PayloadError::UnknownType {
                    type_url: any.type_url.clone(),
                })
            }

            fn type_urls() -> Vec<String> {
                vec![#(#type_urls),*]
            }
        }
    }
}

/// One field: its Rust name, its proto name (for errors) and its shape.
struct Field {
    name: syn::Ident,
    proto_name: String,
    shape: Shape,
}

impl Field {
    fn of(field: &FieldDescriptor, wire: &syn::Path) -> Field {
        let shape = if field.is_map() {
            let Kind::Message(entry) = field.kind() else {
                unreachable!("a map field's kind is its entry message")
            };
            Shape::Map(
                Elem::of(&entry.map_entry_key_field(), wire).domain_type(),
                Elem::of(&entry.map_entry_value_field(), wire),
            )
        } else if field.is_list() {
            Shape::Repeated(Elem::of(field, wire))
        } else if field.containing_oneof().is_some_and(|o| o.is_synthetic()) {
            Shape::Optional(Elem::of(field, wire))
        } else {
            Shape::Single(Elem::of(field, wire))
        };
        Field {
            name: ident(field.name()),
            proto_name: field.name().to_string(),
            shape,
        }
    }
}

/// What one value of a field is.
enum Elem {
    /// A scalar: the same Rust type on both sides.
    Scalar(TokenStream),
    /// A `google.protobuf.Timestamp`: annals' `Timestamp` on both sides.
    Timestamp,
    /// A domain enum; `i32` on the wire.
    Enum(syn::Ident),
    /// A domain message, and its wire type.
    Message(syn::Ident, TokenStream),
}

impl Elem {
    fn of(field: &FieldDescriptor, wire: &syn::Path) -> Elem {
        match field.kind() {
            Kind::Message(m) if m.full_name() == TIMESTAMP => Elem::Timestamp,
            Kind::Message(m) => {
                let name = ident(m.name());
                Elem::Message(name.clone(), quote!(#wire::#name))
            }
            Kind::Enum(e) => Elem::Enum(ident(e.name())),
            Kind::Double => Elem::Scalar(quote!(f64)),
            Kind::Float => Elem::Scalar(quote!(f32)),
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Elem::Scalar(quote!(i32)),
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Elem::Scalar(quote!(i64)),
            Kind::Uint32 | Kind::Fixed32 => Elem::Scalar(quote!(u32)),
            Kind::Uint64 | Kind::Fixed64 => Elem::Scalar(quote!(u64)),
            Kind::Bool => Elem::Scalar(quote!(bool)),
            Kind::String => Elem::Scalar(quote!(String)),
            Kind::Bytes => Elem::Scalar(quote!(Vec<u8>)),
        }
    }

    fn domain_type(&self) -> TokenStream {
        let next = next();
        match self {
            Elem::Scalar(ty) => ty.clone(),
            Elem::Timestamp => quote!(#next::Timestamp),
            Elem::Enum(name) | Elem::Message(name, _) => quote!(#name),
        }
    }
}

/// How a field holds its values.
enum Shape {
    /// One value. A message is required: the wire's absence is refused.
    Single(Elem),
    /// proto3 `optional`: `Option` on both sides.
    Optional(Elem),
    Repeated(Elem),
    /// A map, by its key's Rust type.
    Map(TokenStream, Elem),
}

impl Shape {
    fn domain_type(&self) -> TokenStream {
        match self {
            Shape::Single(e) => e.domain_type(),
            Shape::Optional(e) => {
                let t = e.domain_type();
                quote!(Option<#t>)
            }
            Shape::Repeated(e) => {
                let t = e.domain_type();
                quote!(Vec<#t>)
            }
            Shape::Map(k, e) => {
                let t = e.domain_type();
                quote!(::std::collections::BTreeMap<#k, #t>)
            }
        }
    }

    /// `value`, a domain value of this shape, as its wire value.
    fn encoded(&self, value: TokenStream) -> TokenStream {
        match self {
            Shape::Single(Elem::Scalar(_)) => value,
            Shape::Single(Elem::Timestamp) => quote!(Some(#value)),
            Shape::Single(Elem::Enum(_)) => quote!(i32::from(#value)),
            Shape::Single(Elem::Message(_, w)) => quote!(Some(#w::from(#value))),
            Shape::Optional(Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Optional(Elem::Enum(_)) => quote!(#value.map(i32::from)),
            Shape::Optional(Elem::Message(_, w)) => quote!(#value.map(#w::from)),
            Shape::Repeated(Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Repeated(Elem::Enum(_)) => quote!(#value.into_iter().map(i32::from).collect()),
            Shape::Repeated(Elem::Message(_, w)) => {
                quote!(#value.into_iter().map(#w::from).collect())
            }
            Shape::Map(_, Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Map(_, Elem::Enum(_)) => {
                quote!(#value.into_iter().map(|(k, v)| (k, i32::from(v))).collect())
            }
            Shape::Map(_, Elem::Message(_, w)) => {
                quote!(#value.into_iter().map(|(k, v)| (k, #w::from(v))).collect())
            }
        }
    }

    /// `value`, a wire value of this shape, as its domain value, refusing
    /// what the domain type cannot hold.
    fn decoded(&self, value: TokenStream, message: &str, field: &str) -> TokenStream {
        let next = next();
        let missing = quote! {
            #next::WireError::Missing { message: #message, field: #field }
        };
        let unknown = quote! {
            |value| #next::WireError::UnknownEnum { message: #message, field: #field, value }
        };
        match self {
            Shape::Single(Elem::Scalar(_)) => value,
            Shape::Single(Elem::Timestamp) => quote!(#value.ok_or(#missing)?),
            Shape::Single(Elem::Enum(e)) => quote!(#e::try_from(#value).map_err(#unknown)?),
            Shape::Single(Elem::Message(d, _)) => quote!(#d::try_from(#value.ok_or(#missing)?)?),
            Shape::Optional(Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Optional(Elem::Enum(e)) => {
                quote!(#value.map(|v| #e::try_from(v).map_err(#unknown)).transpose()?)
            }
            Shape::Optional(Elem::Message(d, _)) => quote!(#value.map(#d::try_from).transpose()?),
            Shape::Repeated(Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Repeated(Elem::Enum(e)) => quote! {
                #value
                    .into_iter()
                    .map(|v| #e::try_from(v).map_err(#unknown))
                    .collect::<Result<Vec<_>, _>>()?
            },
            Shape::Repeated(Elem::Message(d, _)) => quote! {
                #value.into_iter().map(#d::try_from).collect::<Result<Vec<_>, _>>()?
            },
            Shape::Map(_, Elem::Scalar(_) | Elem::Timestamp) => value,
            Shape::Map(_, Elem::Enum(e)) => quote! {
                #value
                    .into_iter()
                    .map(|(k, v)| #e::try_from(v).map(|v| (k, v)).map_err(#unknown))
                    .collect::<Result<::std::collections::BTreeMap<_, _>, _>>()?
            },
            Shape::Map(_, Elem::Message(d, _)) => quote! {
                #value
                    .into_iter()
                    .map(|(k, v)| #d::try_from(v).map(|v| (k, v)))
                    .collect::<Result<::std::collections::BTreeMap<_, _>, _>>()?
            },
        }
    }
}

/// `name` as an identifier, raw where it is a Rust keyword (`type`).
pub(crate) fn ident(name: &str) -> syn::Ident {
    syn::parse_str::<syn::Ident>(name)
        .unwrap_or_else(|_| syn::Ident::new_raw(name, Span::call_site()))
}

/// `BuyInWallet` as `buy_in_wallet`: a word starts at an uppercase letter
/// after a lowercase one or a digit, or before a lowercase one in a run of
/// uppercase (`HTTPServer` as `http_server`).
pub(crate) fn snake(camel: &str) -> String {
    let chars: Vec<char> = camel.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let before = chars[i - 1];
            let after = chars.get(i + 1);
            if before.is_ascii_lowercase()
                || before.is_ascii_digit()
                || (before.is_ascii_uppercase() && after.is_some_and(char::is_ascii_lowercase))
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// `CartState` as `CART_STATE`.
fn screaming(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// `SHIPPED_LATE` as `ShippedLate`.
fn camel(screaming: &str) -> String {
    screaming
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            let mut chars = lower.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}
