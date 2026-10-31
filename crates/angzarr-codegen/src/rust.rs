//! The Rust emitter: a package's domain layer for marked-up components on
//! angzarr-embedded ([`markup_package`]).
//!
//! Every domain type is a plain Rust type with its wire conversions: into the
//! prost message infallibly, back out fallibly with angzarr-embedded's `WireError`
//! naming the field. Message fields are required (the wire's `Option` is
//! gone), proto3 `optional` stays an `Option`, and enums are closed: the
//! zero value (`*_UNSPECIFIED`) and any number without a variant are refused.
//! Enums convert by number, so nothing here depends on how prost names
//! variants.

use proc_macro2::{Span, TokenStream};
use prost_reflect::{EnumDescriptor, FieldDescriptor, Kind, MessageDescriptor};
use quote::{format_ident, quote};

use crate::common::{
    enum_name, has_code_status, refusal_choice_name, refusal_key, refusal_reason, screaming,
    wire_name,
};
use crate::model::{Package, TIMESTAMP};

/// The runtime crate the generated code targets, as its Rust identifier:
/// every path into it, the macros crate's name (`<RUNTIME>_macros`) and the
/// prose naming it in generated docs are built from this one name.
pub(crate) const RUNTIME: &str = "angzarr_embedded";

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
        let anchor = package.components.iter().any(|c| &c.anchor == message);
        items.extend(structure(message, wire, anchor));
    }
    items.extend(rejection());
    items.extend(handler_context());
    let mut refusals: Vec<&MessageDescriptor> = Vec::new();
    for component in &package.components {
        for refusal in component.inputs.iter().flat_map(|i| &i.refuses) {
            if !refusals.contains(&refusal) {
                refusals.push(refusal);
            }
        }
        // A projector decides nothing: no command or event enum.
        if component.kind == crate::model::ComponentKind::Projector {
            continue;
        }
        let commands: Vec<MessageDescriptor> = component
            .inputs
            .iter()
            .filter(|i| !i.fact)
            .map(|i| i.message.clone())
            .collect();
        items.extend(plain_enum(
            &format_ident!("{}Command", component.name),
            &commands,
            &format!(" A command of {}, as a policy sends it.", component.name),
        ));
        items.extend(plain_enum(
            &format_ident!("{}Event", component.name),
            &component.events,
            &format!(
                " An event of {}, for a decision recording any number of them.",
                component.name
            ),
        ));
    }
    for refusal in refusals {
        items.extend(refusal_diagnostic(refusal));
    }
    for component in &package.components {
        for input in component.inputs.iter().filter(|i| i.refuses.len() > 1) {
            items.extend(refusal_choice(input));
        }
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
    items.extend(wire_settings(&package.messages, wire));
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

/// The prost settings the domain layer is generated for, checked where the
/// wire types are compiled: a `bytes` field is a `Vec<u8>` (prost's
/// default) and a map a `BTreeMap` (`btree_map(["."])`), as
/// `angzarr_build::wire` builds them. A prost crate built otherwise fails
/// to compile with the named error, `angzarr-codegen-wire-bytes` or
/// `angzarr-codegen-wire-map`, at the field, instead of a type mismatch
/// deep in a conversion. Nothing is emitted for messages holding neither.
fn wire_settings(messages: &[MessageDescriptor], wire: &syn::Path) -> TokenStream {
    let (mut bytes, mut maps) = (false, false);
    let mut checks = Vec::new();
    for message in messages {
        let name = ident(message.name());
        let mut fields = Vec::new();
        for field in message.fields() {
            let f = ident(field.name());
            if field.is_map() {
                maps = true;
                fields.push(quote!(wire_settings::map(&wire.#f);));
            } else if field.kind() == Kind::Bytes {
                bytes = true;
                // A list's or an `optional`'s each value.
                let several =
                    field.is_list() || field.containing_oneof().is_some_and(|o| o.is_synthetic());
                fields.push(if several {
                    quote!(wire.#f.iter().for_each(wire_settings::bytes);)
                } else {
                    quote!(wire_settings::bytes(&wire.#f);)
                });
            }
        }
        if !fields.is_empty() {
            checks.push(quote! {
                const _: fn(&#wire::#name) = |wire| {
                    #(#fields)*
                };
            });
        }
    }
    if checks.is_empty() {
        return TokenStream::new();
    }
    let bytes = bytes.then(|| {
        quote! {
            #[diagnostic::on_unimplemented(
                message = "angzarr-codegen-wire-bytes: a bytes field of the prost types is `{Self}`; the domain layer is generated for `Vec<u8>`",
                label = "not `Vec<u8>`",
                note = "build the prost types with angzarr_build::wire, or prost-build with its default bytes"
            )]
            pub trait Bytes {}

            impl Bytes for Vec<u8> {}

            pub fn bytes<T: Bytes>(_: &T) {}
        }
    });
    let maps = maps.then(|| {
        quote! {
            #[diagnostic::on_unimplemented(
                message = "angzarr-codegen-wire-map: a map field of the prost types is `{Self}`; the domain layer is generated for `BTreeMap`",
                label = "not a `BTreeMap`",
                note = "build the prost types with angzarr_build::wire, or prost-build with btree_map([\".\"])"
            )]
            pub trait Map {}

            impl<K, V> Map for ::std::collections::BTreeMap<K, V> {}

            pub fn map<T: Map>(_: &T) {}
        }
    });
    quote! {
        /// The prost settings this layer is generated for, checked against
        /// the wire types: bytes as `Vec<u8>`, maps as `BTreeMap`.
        mod wire_settings {
            #bytes
            #maps
        }

        #(#checks)*
    }
}

/// The domain's untyped rejection: a diagnostic key and its args, never
/// text (decision 14.37). A command with no typed refusal rejects with it.
fn rejection() -> TokenStream {
    quote! {
        /// Why a command is rejected: a business "no", never a fault, the
        /// command refused under a business status (this one crosses as
        /// FAILED_PRECONDITION) and the issuing policy's refusal handler
        /// told. One of the two classes a handler fails
        /// with; the other is [`Bug`]. A diagnostic key and its args,
        /// never text.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Rejection {
            pub key: &'static str,
            pub args: Vec<(&'static str, String)>,
        }

        impl Rejection {
            /// A rejection keyed `key`, with no args yet.
            pub fn new(key: &'static str) -> Rejection {
                Rejection { key, args: Vec::new() }
            }

            /// This rejection with the arg `name` set to `value`.
            pub fn arg(mut self, name: &'static str, value: impl ToString) -> Rejection {
                self.args.push((name, value.to_string()));
                self
            }
        }

        impl ::std::fmt::Display for Rejection {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.key)
            }
        }

        impl ::std::error::Error for Rejection {}

        impl ::angzarr_embedded_diagnostic::Diagnostic for Rejection {
            fn code(&self) -> &'static str {
                self.key
            }

            fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
                self.args
                    .iter()
                    .map(|(name, value)| (*name, value.as_str().into()))
                    .collect()
            }
        }
    }
}

/// What the user's handlers read and fail with, under the domain layer's
/// names, so a user file imports only `crate::domain`: the runtime's
/// contexts and its `Failed`, `BusinessStatus` and `RetryableStatus`
/// re-exported, and `Bug`,
/// a defect: a diagnostic key and its args, never text (decision 14.37).
fn handler_context() -> TokenStream {
    let next = next();
    quote! {
        /// What a command handler reads besides its command and state: the
        /// instant its events are recorded at, and the router's context of
        /// the input (its stream, sequences, resolution, conflict and
        /// identity) as `ctx.command`.
        pub use #next::markup::Context;

        /// What a policy reaction reads: its trigger, the trigger's
        /// correlation and when it was recorded.
        pub use #next::markup::PolicyContext;

        /// The refused command's context, whole, as its target's handler
        /// was handed it: what a `<command>_refused` method reads.
        pub use #next::markup::CommandContext;

        /// What a projector's handler reads besides the event: the stream
        /// it is from (its source domain and root), its sequence there and
        /// when it was recorded.
        pub use #next::markup::ProjectionContext;

        /// What a command handler fails with, one of four classes:
        /// `Failed::Rejected`, a [`Rejection`] (or a typed refusal) under
        /// a [`BusinessStatus`], FAILED_PRECONDITION unless `with_status`
        /// says otherwise; `Failed::immediate(&bug)`, ABORTED under the
        /// [`Bug`]'s key, "re-read and re-decide", back to a caller at
        /// once and re-run as a policy's step; `Failed::Bug`, a [`Bug`],
        /// dead-lettered at once; or `Failed::retryable_bug(status, &bug)`,
        /// a [`Bug`] under the [`RetryableStatus`] the handler picks,
        /// retried with capped backoff, then dead-lettered.
        /// `Err(rejection)?` and `Err(bug)?` both convert.
        ///
        /// The retryable statuses are not interchangeable: `Unavailable`
        /// means resend (the dependency is down for now);
        /// `DeadlineExceeded` means resend only if the call is idempotent,
        /// since the work may have happened (a command is, by its
        /// `command_id`); `ResourceExhausted` means wait longer before
        /// resending (a quota or a capacity limit was hit).
        pub use #next::markup::{BusinessStatus, Failed, RetryableStatus};

        /// A bug: a defect, never a business answer. The other of the two
        /// classes a handler fails with; the first is [`Rejection`], a
        /// business "no". A command handler fails with it as
        /// `Failed::Bug` (`Err(Bug::new("key").arg(..))?`), a policy as its
        /// `Err`, since a policy reacts to events the application's own
        /// domains recorded. A diagnostic key and its args, never text; it
        /// goes out as ROUTER_REASON_UNHANDLED_HANDLER_ERROR, the key its
        /// `cause`, and the runtime alerts it and dead-letters the input.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Bug {
            pub key: &'static str,
            pub args: Vec<(&'static str, String)>,
        }

        impl Bug {
            /// A bug keyed `key`, with no args yet.
            pub fn new(key: &'static str) -> Bug {
                Bug { key, args: Vec::new() }
            }

            /// This bug with the arg `name` set to `value`.
            pub fn arg(mut self, name: &'static str, value: impl ToString) -> Bug {
                self.args.push((name, value.to_string()));
                self
            }
        }

        impl ::std::fmt::Display for Bug {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.key)
            }
        }

        impl ::std::error::Error for Bug {}

        impl ::angzarr_embedded_diagnostic::Diagnostic for Bug {
            fn code(&self) -> &'static str {
                self.key
            }

            fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
                self.args
                    .iter()
                    .map(|(name, value)| (*name, value.as_str().into()))
                    .collect()
            }
        }

        /// A command handler's bug, as it fails: `Failed::Bug`.
        impl ::core::convert::From<Bug> for Failed<Rejection> {
            fn from(bug: Bug) -> Self {
                Failed::bug(&bug)
            }
        }
    }
}

/// A typed refusal as a diagnostic (decision 14.50): its code
/// [`refusal_key`] of the message's full name and, when it declares a
/// nested `Reason` enum as its `reason` field, the reason's wire name; its
/// args the fields that are set, each by its proto name (see
/// [`field_arg`]).
fn refusal_diagnostic(message: &MessageDescriptor) -> TokenStream {
    let name = ident(message.name());
    let code = match refusal_reason(message) {
        Some(e) => {
            let ty = ident(&enum_name(&e));
            let arms = e.values().filter(|v| v.number() != 0).map(|v| {
                let variant = ident(&camel(wire_name(&e, v.name())));
                let key = refusal_key(message.full_name(), Some(wire_name(&e, v.name())));
                quote!(#ty::#variant => #key)
            });
            quote!(match self.reason { #(#arms),* })
        }
        _ => {
            let key = refusal_key(message.full_name(), None);
            quote!(#key)
        }
    };
    let args: Vec<TokenStream> = message.fields().filter_map(|f| field_arg(&f)).collect();
    let args = if args.is_empty() {
        quote!(Vec::new())
    } else {
        quote! {
            let mut args = Vec::new();
            #(#args)*
            args
        }
    };
    let status = refusal_reason(message).map(|e| {
        let ty = ident(&enum_name(&e));
        let arms = e.values().filter(|v| v.number() != 0).map(|v| {
            let variant = ident(&camel(wire_name(&e, v.name())));
            let status = business_status(crate::model::refusal_status(&v));
            quote!(#ty::#variant => BusinessStatus::#status)
        });
        quote! {
            impl #name {
                /// The status this refusal is refused with (decision 14.52): its
                /// reason's `refusal_status`, FAILED_PRECONDITION where unmarked.
                pub fn status(&self) -> BusinessStatus {
                    match self.reason { #(#arms),* }
                }

                /// `failed`, its refusal under its code's status ([`Self::status`]),
                /// whatever status it was given; any other failure unchanged.
                /// What the generated adapter refuses with.
                pub fn with_code_status(failed: Failed<#name>) -> Failed<#name> {
                    match failed {
                        Failed::Rejected { rejection, .. } => {
                            let status = rejection.status();
                            Failed::Rejected { rejection, status }
                        }
                        other => other,
                    }
                }
            }
        }
    });
    quote! {
        #status

        /// A bug of the handler refusing with this, as it fails:
        /// `Failed::Bug`.
        impl ::core::convert::From<Bug> for Failed<#name> {
            fn from(bug: Bug) -> Self {
                Failed::bug(&bug)
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
            }
        }

        impl ::angzarr_embedded_diagnostic::Diagnostic for #name {
            fn code(&self) -> &'static str {
                #code
            }

            fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
                #args
            }
        }
    }
}

/// What a command naming several `refuses` types refuses with: one of
/// them, each `From` its type, its diagnostic the chosen one's, and
/// `widen`, which hands the adapter's markup its own choice.
fn refusal_choice(input: &crate::model::Input) -> TokenStream {
    let name = format_ident!("{}", refusal_choice_name(&input.message));
    let types: Vec<syn::Ident> = input.refuses.iter().map(|r| ident(r.name())).collect();
    let doc = format!(
        " What {} refuses with: one of its refuses types.",
        input.message.name()
    );
    let coded = has_code_status(input).then(|| {
        let arms = input.refuses.iter().map(|r| {
            let ty = ident(r.name());
            if refusal_reason(r).is_some() {
                quote!(#name::#ty(refusal) => refusal.status())
            } else {
                quote!(#name::#ty(_) => status)
            }
        });
        quote! {
            impl #name {
                /// `failed`, its refusal under the chosen type's code's status
                /// (decision 14.52) when it has a Reason, else the status it was
                /// given; any other failure unchanged. What the generated adapter
                /// refuses with.
                pub fn with_code_status(failed: Failed<#name>) -> Failed<#name> {
                    match failed {
                        Failed::Rejected { rejection, status } => {
                            let status = match &rejection { #(#arms),* };
                            Failed::Rejected { rejection, status }
                        }
                        other => other,
                    }
                }
            }
        }
    });
    quote! {
        #coded

        #[doc = #doc]
        #[derive(Clone, Debug, PartialEq)]
        pub enum #name { #(#types(#types)),* }

        #(
            impl From<#types> for #name {
                fn from(refusal: #types) -> Self {
                    #name::#types(refusal)
                }
            }
        )*

        impl #name {
            /// `failed`, its refusal converted by the chosen type's own
            /// `Into`: what the generated adapter hands the markup.
            pub fn widen<Y>(failed: Failed<#name>) -> Failed<Y>
            where
                #(#types: Into<Y>,)*
            {
                match failed {
                    Failed::Rejected { rejection, status } => Failed::Rejected {
                        rejection: match rejection {
                            #(#name::#types(refusal) => refusal.into(),)*
                        },
                        status,
                    },
                    Failed::Immediate(report) => Failed::Immediate(report),
                    Failed::Bug(report) => Failed::Bug(report),
                    Failed::RetryableBug { status, report } => Failed::RetryableBug { status, report },
                }
            }
        }

        /// A bug of the handler refusing with this, as it fails:
        /// `Failed::Bug`.
        impl ::core::convert::From<Bug> for Failed<#name> {
            fn from(bug: Bug) -> Self {
                Failed::bug(&bug)
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#name::#types(refusal) => ::std::fmt::Display::fmt(refusal, f),)*
                }
            }
        }

        impl ::angzarr_embedded_diagnostic::Diagnostic for #name {
            fn code(&self) -> &'static str {
                match self {
                    #(#name::#types(refusal) => ::angzarr_embedded_diagnostic::Diagnostic::code(refusal),)*
                }
            }

            fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
                match self {
                    #(#name::#types(refusal) => ::angzarr_embedded_diagnostic::Diagnostic::args(refusal),)*
                }
            }
        }
    }
}

/// The `BusinessStatus` variant of a `refusal_status` number (a
/// `google.rpc.Code`, decision 14.52); unmarked, or a number outside the
/// business set, FAILED_PRECONDITION.
fn business_status(code: i32) -> syn::Ident {
    format_ident!(
        "{}",
        match code {
            3 => "InvalidArgument",
            5 => "NotFound",
            6 => "AlreadyExists",
            7 => "PermissionDenied",
            11 => "OutOfRange",
            _ => "FailedPrecondition",
        }
    )
}

/// The statement pushing `field` onto a refusal's args when it is set:
/// a proto3 `optional` one when it is `Some`, any other always. An
/// integer is its number, a string itself, an enum its value's wire
/// name ([`wire_name`]), bytes their lower-case hex, a bool or a float
/// its Rust text. A message, list or map field is not an arg (`None`).
fn field_arg(field: &FieldDescriptor) -> Option<TokenStream> {
    if field.is_list() || field.is_map() {
        return None;
    }
    let arg = quote!(::angzarr_embedded_diagnostic::Arg);
    let value = match field.kind() {
        Kind::Message(_) => return None,
        Kind::Enum(e) => {
            let ty = ident(&enum_name(&e));
            let arms = e.values().filter(|v| v.number() != 0).map(|v| {
                let variant = ident(&camel(wire_name(&e, v.name())));
                let wire = wire_name(&e, v.name());
                quote!(#ty::#variant => #wire)
            });
            quote!(#arg::from(match value { #(#arms),* }))
        }
        Kind::String => quote!(#arg::from(value.as_str())),
        Kind::Bytes => quote! {
            #arg::from(value.iter().map(|b| format!("{b:02x}")).collect::<String>())
        },
        Kind::Bool | Kind::Float | Kind::Double => quote!(#arg::from(value.to_string())),
        _ => quote!(#arg::from(*value)),
    };
    let name = ident(field.name());
    let proto = field.name();
    Some(
        if field.containing_oneof().is_some_and(|o| o.is_synthetic()) {
            quote! {
                if let Some(value) = &self.#name {
                    args.push((#proto, #value));
                }
            }
        } else {
            quote! {
                let value = &self.#name;
                args.push((#proto, #value));
            }
        },
    )
}

/// An enum of `messages`, one variant each, with no wire form.
fn plain_enum(name: &syn::Ident, messages: &[MessageDescriptor], doc: &str) -> TokenStream {
    let variants: Vec<syn::Ident> = messages.iter().map(|m| ident(m.name())).collect();
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
        #[doc = #doc]
        #[derive(Clone, Debug, PartialEq)]
        pub enum #name { #(#variants(#variants)),* }

        #(#froms)*
    }
}

fn enumeration(e: &EnumDescriptor) -> TokenStream {
    let name = ident(&enum_name(e));
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
    /// A `google.protobuf.Timestamp`: angzarr-embedded's `Timestamp` on both sides.
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
            Kind::Enum(e) => Elem::Enum(ident(&enum_name(&e))),
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
