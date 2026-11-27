//! The terminal states every owner journal (adoption, approval, definition,
//! library) can record. Each journal keeps its own public type, and so its own
//! name in the generated contract, but all four are defined here once.

/// How a journal entry ended, independent of which journal recorded it.
/// Only the persistence layer reads it, so builds without that feature skip it.
#[cfg_attr(not(feature = "persistence"), allow(dead_code))]
pub(crate) trait JournalOutcome: Sized {
    const SUCCEEDED: Self;
    const REJECTED: Self;
    const REVIEWED_UNKNOWN: Self;
}

/// Defines a journal outcome enum with the shared variants, wire format and
/// [`JournalOutcome`] implementation.
macro_rules! journal_outcome {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            serde::Serialize,
            serde::Deserialize,
            schemars::JsonSchema,
        )]
        #[serde(rename_all = "kebab-case")]
        pub enum $name {
            Succeeded,
            Rejected,
            ReviewedUnknown,
        }

        impl $crate::control::owner_outcome::JournalOutcome for $name {
            const SUCCEEDED: Self = Self::Succeeded;
            const REJECTED: Self = Self::Rejected;
            const REVIEWED_UNKNOWN: Self = Self::ReviewedUnknown;
        }
    };
}
pub(crate) use journal_outcome;

/// Defines the record a journal keeps for a finished entry: the frozen intent,
/// how it ended, the journal-specific result (`$result`, a receipt or a
/// revision), the error code for rejections and when it was recorded.
macro_rules! journal_record {
    ($name:ident { intent: $intent:ty, outcome: $outcome:ty, $result:ident: $result_ty:ty }) => {
        #[derive(
            Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
        )]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name {
            pub intent: $intent,
            pub outcome: $outcome,
            pub $result: $result_ty,
            pub error_code: Option<String>,
            pub recorded_at: i64,
        }
    };
}
pub(crate) use journal_record;
