use std::num::TryFromIntError;
use std::path::Path;

use fugue_core::ir::{Address, CodeBlockId};
use fugue_core::storage::SegmentStorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuokkaBuilderError {
    #[error("failed to encode protobuf: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("I/O error while {action} `{path}`: {source}")]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("I/O error while writing stream: {0}")]
    Stream(#[source] std::io::Error),
    #[error("failed to inspect segments: {0}")]
    Segment(#[from] SegmentStorageError),
    #[error("numeric value for `{field}` is out of range")]
    Numeric {
        field: &'static str,
        #[source]
        source: TryFromIntError,
    },
    #[error("code block {block:?} referenced by function at {function} was not found")]
    MissingBlock {
        function: Address,
        block: CodeBlockId,
    },
}

impl QuokkaBuilderError {
    pub(crate) fn io(action: &'static str, path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            action,
            path: path.display().to_string(),
            source,
        }
    }

    pub(crate) fn numeric(field: &'static str, source: TryFromIntError) -> Self {
        Self::Numeric { field, source }
    }

    pub(crate) fn with_path(self, action: &'static str, path: &Path) -> Self {
        match self {
            Self::Stream(source) => Self::io(action, path, source),
            error => error,
        }
    }
}
