#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PostRefusedReason {
    Closed,
    OverLimit,
}
impl From<PostRefusedReason> for i32 {
    fn from(value: PostRefusedReason) -> i32 {
        match value {
            PostRefusedReason::Closed => 1i32,
            PostRefusedReason::OverLimit => 2i32,
        }
    }
}
impl TryFrom<i32> for PostRefusedReason {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            1i32 => Ok(PostRefusedReason::Closed),
            2i32 => Ok(PostRefusedReason::OverLimit),
            other => Err(other),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tone {
    Loud,
}
impl From<Tone> for i32 {
    fn from(value: Tone) -> i32 {
        match value {
            Tone::Loud => 1i32,
        }
    }
}
impl TryFrom<i32> for Tone {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            1i32 => Ok(Tone::Loud),
            other => Err(other),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ListedReason {
    Full,
}
impl From<ListedReason> for i32 {
    fn from(value: ListedReason) -> i32 {
        match value {
            ListedReason::Full => 1i32,
        }
    }
}
impl TryFrom<i32> for ListedReason {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            1i32 => Ok(ListedReason::Full),
            other => Err(other),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElsewhereReason {
    Away,
}
impl From<ElsewhereReason> for i32 {
    fn from(value: ElsewhereReason) -> i32 {
        match value {
            ElsewhereReason::Away => 1i32,
        }
    }
}
impl TryFrom<i32> for ElsewhereReason {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            1i32 => Ok(ElsewhereReason::Away),
            other => Err(other),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MislabeledCause {
    Late,
}
impl From<MislabeledCause> for i32 {
    fn from(value: MislabeledCause) -> i32 {
        match value {
            MislabeledCause::Late => 1i32,
        }
    }
}
impl TryFrom<i32> for MislabeledCause {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            1i32 => Ok(MislabeledCause::Late),
            other => Err(other),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Post {}
impl From<Post> for crate::wire::Post {
    fn from(_value: Post) -> Self {
        crate::wire::Post {}
    }
}
impl TryFrom<crate::wire::Post> for Post {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(_value: crate::wire::Post) -> Result<Self, Self::Error> {
        Ok(Post {})
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PostRefused {
    pub reason: PostRefusedReason,
    pub small: i32,
    pub big: i64,
    pub count: u32,
    pub total: u64,
    pub text: String,
    pub data: Vec<u8>,
    pub flag: bool,
    pub ratio: f64,
    pub share: f32,
    pub tone: Tone,
    pub limit: Option<i64>,
    pub maybe: Option<Tone>,
    pub note: Note,
    pub list: Vec<i32>,
    pub table: ::std::collections::BTreeMap<String, i32>,
}
impl From<PostRefused> for crate::wire::PostRefused {
    fn from(value: PostRefused) -> Self {
        crate::wire::PostRefused {
            reason: i32::from(value.reason),
            small: value.small,
            big: value.big,
            count: value.count,
            total: value.total,
            text: value.text,
            data: value.data,
            flag: value.flag,
            ratio: value.ratio,
            share: value.share,
            tone: i32::from(value.tone),
            limit: value.limit,
            maybe: value.maybe.map(i32::from),
            note: Some(crate::wire::Note::from(value.note)),
            list: value.list,
            table: value.table,
        }
    }
}
impl TryFrom<crate::wire::PostRefused> for PostRefused {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::PostRefused) -> Result<Self, Self::Error> {
        Ok(PostRefused {
            reason: PostRefusedReason::try_from(value.reason)
                .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                    message: "kinds.v1.PostRefused",
                    field: "reason",
                    value,
                })?,
            small: value.small,
            big: value.big,
            count: value.count,
            total: value.total,
            text: value.text,
            data: value.data,
            flag: value.flag,
            ratio: value.ratio,
            share: value.share,
            tone: Tone::try_from(value.tone)
                .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                    message: "kinds.v1.PostRefused",
                    field: "tone",
                    value,
                })?,
            limit: value.limit,
            maybe: value
                .maybe
                .map(|v| {
                    Tone::try_from(v)
                        .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                            message: "kinds.v1.PostRefused",
                            field: "maybe",
                            value,
                        })
                })
                .transpose()?,
            note: Note::try_from(
                value
                    .note
                    .ok_or(::angzarr_embedded::next::WireError::Missing {
                        message: "kinds.v1.PostRefused",
                        field: "note",
                    })?,
            )?,
            list: value.list,
            table: value.table,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
}
impl From<Note> for crate::wire::Note {
    fn from(value: Note) -> Self {
        crate::wire::Note {
            text: value.text,
        }
    }
}
impl TryFrom<crate::wire::Note> for Note {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::Note) -> Result<Self, Self::Error> {
        Ok(Note { text: value.text })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Unreasoned {
    pub reason: Tone,
}
impl From<Unreasoned> for crate::wire::Unreasoned {
    fn from(value: Unreasoned) -> Self {
        crate::wire::Unreasoned {
            reason: i32::from(value.reason),
        }
    }
}
impl TryFrom<crate::wire::Unreasoned> for Unreasoned {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::Unreasoned) -> Result<Self, Self::Error> {
        Ok(Unreasoned {
            reason: Tone::try_from(value.reason)
                .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                    message: "kinds.v1.Unreasoned",
                    field: "reason",
                    value,
                })?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub reason: Vec<ListedReason>,
}
impl From<Listed> for crate::wire::Listed {
    fn from(value: Listed) -> Self {
        crate::wire::Listed {
            reason: value.reason.into_iter().map(i32::from).collect(),
        }
    }
}
impl TryFrom<crate::wire::Listed> for Listed {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::Listed) -> Result<Self, Self::Error> {
        Ok(Listed {
            reason: value
                .reason
                .into_iter()
                .map(|v| {
                    ListedReason::try_from(v)
                        .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                            message: "kinds.v1.Listed",
                            field: "reason",
                            value,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Elsewhere {
    pub why: ElsewhereReason,
}
impl From<Elsewhere> for crate::wire::Elsewhere {
    fn from(value: Elsewhere) -> Self {
        crate::wire::Elsewhere {
            why: i32::from(value.why),
        }
    }
}
impl TryFrom<crate::wire::Elsewhere> for Elsewhere {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::Elsewhere) -> Result<Self, Self::Error> {
        Ok(Elsewhere {
            why: ElsewhereReason::try_from(value.why)
                .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                    message: "kinds.v1.Elsewhere",
                    field: "why",
                    value,
                })?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Mislabeled {
    pub reason: MislabeledCause,
}
impl From<Mislabeled> for crate::wire::Mislabeled {
    fn from(value: Mislabeled) -> Self {
        crate::wire::Mislabeled {
            reason: i32::from(value.reason),
        }
    }
}
impl TryFrom<crate::wire::Mislabeled> for Mislabeled {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(value: crate::wire::Mislabeled) -> Result<Self, Self::Error> {
        Ok(Mislabeled {
            reason: MislabeledCause::try_from(value.reason)
                .map_err(|value| ::angzarr_embedded::next::WireError::UnknownEnum {
                    message: "kinds.v1.Mislabeled",
                    field: "reason",
                    value,
                })?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Posted {}
impl From<Posted> for crate::wire::Posted {
    fn from(_value: Posted) -> Self {
        crate::wire::Posted {}
    }
}
impl TryFrom<crate::wire::Posted> for Posted {
    type Error = ::angzarr_embedded::next::WireError;
    fn try_from(_value: crate::wire::Posted) -> Result<Self, Self::Error> {
        Ok(Posted {})
    }
}
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
        self.args.iter().map(|(name, value)| (*name, value.as_str().into())).collect()
    }
}
/// What a command handler reads besides its command and state: the
/// instant its events are recorded at, and the router's context of
/// the input (its stream, sequences, resolution, conflict and
/// identity) as `ctx.command`.
pub use ::angzarr_embedded::next::markup::Context;
/// What a policy reaction reads: its trigger, the trigger's
/// correlation and when it was recorded.
pub use ::angzarr_embedded::next::markup::PolicyContext;
/// The refused command's context, whole, as its target's handler
/// was handed it: what a `<command>_refused` method reads.
pub use ::angzarr_embedded::next::markup::CommandContext;
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
pub use ::angzarr_embedded::next::markup::{BusinessStatus, Failed, RetryableStatus};
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
        self.args.iter().map(|(name, value)| (*name, value.as_str().into())).collect()
    }
}
/// A command handler's bug, as it fails: `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<Rejection> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
/// A command of LedgerState, as a policy sends it.
#[derive(Clone, Debug, PartialEq)]
pub enum LedgerStateCommand {
    Post(Post),
}
impl From<Post> for LedgerStateCommand {
    fn from(value: Post) -> Self {
        LedgerStateCommand::Post(value)
    }
}
/// An event of LedgerState, for a decision recording any number of them.
#[derive(Clone, Debug, PartialEq)]
pub enum LedgerStateEvent {
    Posted(Posted),
}
impl From<Posted> for LedgerStateEvent {
    fn from(value: Posted) -> Self {
        LedgerStateEvent::Posted(value)
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<PostRefused> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for PostRefused {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for PostRefused {
    fn code(&self) -> &'static str {
        match self.reason {
            PostRefusedReason::Closed => "kinds-v1-postrefused-closed",
            PostRefusedReason::OverLimit => "kinds-v1-postrefused-over-limit",
        }
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        let mut args = Vec::new();
        let value = &self.reason;
        args.push((
            "reason",
            ::angzarr_embedded_diagnostic::Arg::from(
                match value {
                    PostRefusedReason::Closed => "CLOSED",
                    PostRefusedReason::OverLimit => "OVER_LIMIT",
                },
            ),
        ));
        let value = &self.small;
        args.push(("small", ::angzarr_embedded_diagnostic::Arg::from(*value)));
        let value = &self.big;
        args.push(("big", ::angzarr_embedded_diagnostic::Arg::from(*value)));
        let value = &self.count;
        args.push(("count", ::angzarr_embedded_diagnostic::Arg::from(*value)));
        let value = &self.total;
        args.push(("total", ::angzarr_embedded_diagnostic::Arg::from(*value)));
        let value = &self.text;
        args.push(("text", ::angzarr_embedded_diagnostic::Arg::from(value.as_str())));
        let value = &self.data;
        args.push((
            "data",
            ::angzarr_embedded_diagnostic::Arg::from(
                value.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            ),
        ));
        let value = &self.flag;
        args.push(("flag", ::angzarr_embedded_diagnostic::Arg::from(value.to_string())));
        let value = &self.ratio;
        args.push((
            "ratio",
            ::angzarr_embedded_diagnostic::Arg::from(value.to_string()),
        ));
        let value = &self.share;
        args.push((
            "share",
            ::angzarr_embedded_diagnostic::Arg::from(value.to_string()),
        ));
        let value = &self.tone;
        args.push((
            "tone",
            ::angzarr_embedded_diagnostic::Arg::from(
                match value {
                    Tone::Loud => "LOUD",
                },
            ),
        ));
        if let Some(value) = &self.limit {
            args.push(("limit", ::angzarr_embedded_diagnostic::Arg::from(*value)));
        }
        if let Some(value) = &self.maybe {
            args.push((
                "maybe",
                ::angzarr_embedded_diagnostic::Arg::from(
                    match value {
                        Tone::Loud => "LOUD",
                    },
                ),
            ));
        }
        args
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<Unreasoned> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for Unreasoned {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for Unreasoned {
    fn code(&self) -> &'static str {
        "kinds-v1-unreasoned"
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        let mut args = Vec::new();
        let value = &self.reason;
        args.push((
            "reason",
            ::angzarr_embedded_diagnostic::Arg::from(
                match value {
                    Tone::Loud => "LOUD",
                },
            ),
        ));
        args
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<Listed> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for Listed {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for Listed {
    fn code(&self) -> &'static str {
        "kinds-v1-listed"
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        Vec::new()
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<Elsewhere> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for Elsewhere {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for Elsewhere {
    fn code(&self) -> &'static str {
        "kinds-v1-elsewhere"
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        let mut args = Vec::new();
        let value = &self.why;
        args.push((
            "why",
            ::angzarr_embedded_diagnostic::Arg::from(
                match value {
                    ElsewhereReason::Away => "AWAY",
                },
            ),
        ));
        args
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<Mislabeled> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for Mislabeled {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(::angzarr_embedded_diagnostic::Diagnostic::code(self))
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for Mislabeled {
    fn code(&self) -> &'static str {
        "kinds-v1-mislabeled"
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        let mut args = Vec::new();
        let value = &self.reason;
        args.push((
            "reason",
            ::angzarr_embedded_diagnostic::Arg::from(
                match value {
                    MislabeledCause::Late => "LATE",
                },
            ),
        ));
        args
    }
}
/// What Post refuses with: one of its refuses types.
#[derive(Clone, Debug, PartialEq)]
pub enum PostRefuses {
    PostRefused(PostRefused),
    Unreasoned(Unreasoned),
    Listed(Listed),
    Elsewhere(Elsewhere),
    Mislabeled(Mislabeled),
}
impl From<PostRefused> for PostRefuses {
    fn from(refusal: PostRefused) -> Self {
        PostRefuses::PostRefused(refusal)
    }
}
impl From<Unreasoned> for PostRefuses {
    fn from(refusal: Unreasoned) -> Self {
        PostRefuses::Unreasoned(refusal)
    }
}
impl From<Listed> for PostRefuses {
    fn from(refusal: Listed) -> Self {
        PostRefuses::Listed(refusal)
    }
}
impl From<Elsewhere> for PostRefuses {
    fn from(refusal: Elsewhere) -> Self {
        PostRefuses::Elsewhere(refusal)
    }
}
impl From<Mislabeled> for PostRefuses {
    fn from(refusal: Mislabeled) -> Self {
        PostRefuses::Mislabeled(refusal)
    }
}
impl PostRefuses {
    /// `failed`, its refusal converted by the chosen type's own
    /// `Into`: what the generated adapter hands the markup.
    pub fn widen<Y>(failed: Failed<PostRefuses>) -> Failed<Y>
    where
        PostRefused: Into<Y>,
        Unreasoned: Into<Y>,
        Listed: Into<Y>,
        Elsewhere: Into<Y>,
        Mislabeled: Into<Y>,
    {
        match failed {
            Failed::Rejected { rejection, status } => {
                Failed::Rejected {
                    rejection: match rejection {
                        PostRefuses::PostRefused(refusal) => refusal.into(),
                        PostRefuses::Unreasoned(refusal) => refusal.into(),
                        PostRefuses::Listed(refusal) => refusal.into(),
                        PostRefuses::Elsewhere(refusal) => refusal.into(),
                        PostRefuses::Mislabeled(refusal) => refusal.into(),
                    },
                    status,
                }
            }
            Failed::Immediate(report) => Failed::Immediate(report),
            Failed::Bug(report) => Failed::Bug(report),
            Failed::RetryableBug { status, report } => {
                Failed::RetryableBug {
                    status,
                    report,
                }
            }
        }
    }
}
/// A bug of the handler refusing with this, as it fails:
/// `Failed::Bug`.
impl ::core::convert::From<Bug> for Failed<PostRefuses> {
    fn from(bug: Bug) -> Self {
        Failed::bug(&bug)
    }
}
impl ::std::fmt::Display for PostRefuses {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            PostRefuses::PostRefused(refusal) => ::std::fmt::Display::fmt(refusal, f),
            PostRefuses::Unreasoned(refusal) => ::std::fmt::Display::fmt(refusal, f),
            PostRefuses::Listed(refusal) => ::std::fmt::Display::fmt(refusal, f),
            PostRefuses::Elsewhere(refusal) => ::std::fmt::Display::fmt(refusal, f),
            PostRefuses::Mislabeled(refusal) => ::std::fmt::Display::fmt(refusal, f),
        }
    }
}
impl ::angzarr_embedded_diagnostic::Diagnostic for PostRefuses {
    fn code(&self) -> &'static str {
        match self {
            PostRefuses::PostRefused(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::code(refusal)
            }
            PostRefuses::Unreasoned(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::code(refusal)
            }
            PostRefuses::Listed(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::code(refusal)
            }
            PostRefuses::Elsewhere(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::code(refusal)
            }
            PostRefuses::Mislabeled(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::code(refusal)
            }
        }
    }
    fn args(&self) -> Vec<(&'static str, ::angzarr_embedded_diagnostic::Arg)> {
        match self {
            PostRefuses::PostRefused(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::args(refusal)
            }
            PostRefuses::Unreasoned(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::args(refusal)
            }
            PostRefuses::Listed(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::args(refusal)
            }
            PostRefuses::Elsewhere(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::args(refusal)
            }
            PostRefuses::Mislabeled(refusal) => {
                ::angzarr_embedded_diagnostic::Diagnostic::args(refusal)
            }
        }
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Post {
    type Wire = crate::wire::Post;
    fn to_wire(&self) -> crate::wire::Post {
        crate::wire::Post::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Post,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Post::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for PostRefused {
    type Wire = crate::wire::PostRefused;
    fn to_wire(&self) -> crate::wire::PostRefused {
        crate::wire::PostRefused::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::PostRefused,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        PostRefused::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Unreasoned {
    type Wire = crate::wire::Unreasoned;
    fn to_wire(&self) -> crate::wire::Unreasoned {
        crate::wire::Unreasoned::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Unreasoned,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Unreasoned::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Listed {
    type Wire = crate::wire::Listed;
    fn to_wire(&self) -> crate::wire::Listed {
        crate::wire::Listed::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Listed,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Listed::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Elsewhere {
    type Wire = crate::wire::Elsewhere;
    fn to_wire(&self) -> crate::wire::Elsewhere {
        crate::wire::Elsewhere::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Elsewhere,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Elsewhere::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Mislabeled {
    type Wire = crate::wire::Mislabeled;
    fn to_wire(&self) -> crate::wire::Mislabeled {
        crate::wire::Mislabeled::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Mislabeled,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Mislabeled::try_from(wire)
    }
}
impl ::angzarr_embedded::next::markup::DomainMessage for Posted {
    type Wire = crate::wire::Posted;
    fn to_wire(&self) -> crate::wire::Posted {
        crate::wire::Posted::from(self.clone())
    }
    fn from_wire(
        wire: crate::wire::Posted,
    ) -> Result<Self, ::angzarr_embedded::next::WireError> {
        Posted::try_from(wire)
    }
}
