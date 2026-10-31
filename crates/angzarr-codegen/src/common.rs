//! The language-neutral rules every backend shares: how a proto name is
//! cased and localised, and how a typed refusal is keyed (decision 14.50)
//! and whether it carries a code status (decision 14.52). The Rust and the
//! Python backends both depend on this module; neither depends on the
//! other.

use prost_reflect::{EnumDescriptor, Kind, MessageDescriptor};

/// The diagnostic key of a typed refusal (decision 14.50): its message's
/// full name, then the reason's wire name when it has one (its `Reason`
/// value's name without the enum's prefix), `.` and `_` as `-`, all
/// lower-case: `refusals.v1.BetRefused` refusing `REASON_ALREADY_BET` is
/// `refusals-v1-betrefused-already-bet`. The generated `Diagnostic::code`
/// is this; a hand-written refusal reproduces it with this function.
pub fn refusal_key(full_name: &str, reason: Option<&str>) -> String {
    let key = match reason {
        Some(reason) => format!("{full_name}-{reason}"),
        None => full_name.to_string(),
    };
    key.replace(['.', '_'], "-").to_lowercase()
}

/// A typed refusal's rejection codes (decision 14.50): the nested `Reason`
/// enum it declares as its singular `reason` field, which its diagnostic
/// key reads; `None` for a refusal without one.
pub(crate) fn refusal_reason(message: &MessageDescriptor) -> Option<EnumDescriptor> {
    let field = message
        .get_field_by_name("reason")
        .filter(|f| !f.is_list())?;
    match field.kind() {
        Kind::Enum(e) if e.name() == "Reason" && e.parent_message().as_ref() == Some(message) => {
            Some(e)
        }
        _ => None,
    }
}

/// Whether a refusal of `input` is refused under its code's status
/// (decision 14.52): one of its refuses types has a Reason.
pub(crate) fn has_code_status(input: &crate::model::Input) -> bool {
    input.refuses.iter().any(|r| refusal_reason(r).is_some())
}

/// The name of the choice a command refusing several types refuses with
/// (`<Command>Refuses`) in the domain layer, which no message or enum may
/// take.
pub(crate) fn refusal_choice_name(command: &MessageDescriptor) -> String {
    format!("{}Refuses", command.name())
}

/// An enum value's wire name: its name without the enum's prefix
/// (`REASON_ALREADY_BET` of `Reason` as `ALREADY_BET`), what ErrorInfo
/// carries and an arg holds.
pub(crate) fn wire_name<'a>(e: &EnumDescriptor, value: &'a str) -> &'a str {
    let prefix = format!("{}_", screaming(e.name()));
    value.strip_prefix(&prefix).unwrap_or(value)
}

/// A type's plain name: its full name without `package`, its enclosing
/// messages' names, then its own (`SeatOutcome.Outcome` as
/// `SeatOutcomeOutcome`).
pub(crate) fn local_name(full: &str, package: &str) -> String {
    full.strip_prefix(package)
        .map_or(full, |rest| rest.trim_start_matches('.'))
        .replace('.', "")
}

/// An enum's domain name: its [`local_name`].
pub(crate) fn enum_name(e: &EnumDescriptor) -> String {
    local_name(e.full_name(), e.package_name())
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
pub(crate) fn screaming(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}
