// error.rs — VerifierError and ErrorCode for the Evidence Checkpoint Engine verifier.

use thiserror::Error;

/// Semantic error codes for precise classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    UnsupportedVersion,
    Uint64Overflow,
    LeadingZeroIdentifier,
    GitAlgorithmLengthMismatch,
    SelfInjectionAttempt,
    DuplicateJobName,
    InvalidJobStatusConclusion,
    MissingPolicyReference,
    UnverifiedPolicy,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::UnsupportedVersion => write!(f, "UNSUPPORTED_VERSION"),
            ErrorCode::Uint64Overflow => write!(f, "UINT64_OVERFLOW"),
            ErrorCode::LeadingZeroIdentifier => write!(f, "LEADING_ZERO_IDENTIFIER"),
            ErrorCode::GitAlgorithmLengthMismatch => write!(f, "GIT_ALGORITHM_LENGTH_MISMATCH"),
            ErrorCode::SelfInjectionAttempt => write!(f, "SELF_INJECTION_ATTEMPT"),
            ErrorCode::DuplicateJobName => write!(f, "DUPLICATE_JOB_NAME"),
            ErrorCode::InvalidJobStatusConclusion => write!(f, "INVALID_JOB_STATUS_CONCLUSION"),
            ErrorCode::MissingPolicyReference => write!(f, "MISSING_POLICY_REFERENCE"),
            ErrorCode::UnverifiedPolicy => write!(f, "UNVERIFIED_POLICY"),
        }
    }
}

/// All error conditions the verifier can produce.
#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Input exceeds maximum allowed size of 10 MiB")]
    InputTooLarge,

    #[error("JSON nesting depth exceeds maximum of 32")]
    NestingTooDeep,

    #[error("Input is not valid UTF-8")]
    InvalidUtf8,

    #[error("Duplicate JSON object key: {key:?}")]
    DuplicateKey { key: String },

    #[error("Malformed JSON: {detail}")]
    MalformedJson { detail: String },

    #[error("Schema validation failed: {}", details.join("; "))]
    SchemaViolation { details: Vec<String> },

    #[error("Envelope digest mismatch: expected {expected}, computed {computed}")]
    DigestMismatch { expected: String, computed: String },

    #[error("Semantic check failed [{code}]: {detail}")]
    SemanticError { code: ErrorCode, detail: String },

    #[error("Schema file I/O error: {0}")]
    SchemaIo(String),

    #[error("Output assessment schema validation failed: {}", details.join("; "))]
    OutputSchemaViolation { details: Vec<String> },

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl VerifierError {
    /// Map this error to the exit code contract.
    /// 2 = confirmed verification failure
    /// 3 = refused/malformed/invalid/insufficient
    /// 4 = internal execution error
    pub fn exit_code(&self) -> i32 {
        match self {
            VerifierError::DigestMismatch { .. } => 2,
            VerifierError::InternalError(_)
            | VerifierError::SchemaIo(_)
            | VerifierError::OutputSchemaViolation { .. } => 4,
            _ => 3,
        }
    }

    /// Human-readable error class for stderr diagnostics.
    pub fn class(&self) -> &'static str {
        match self {
            VerifierError::InputTooLarge => "INPUT_TOO_LARGE",
            VerifierError::NestingTooDeep => "NESTING_TOO_DEEP",
            VerifierError::InvalidUtf8 => "INVALID_UTF8",
            VerifierError::DuplicateKey { .. } => "DUPLICATE_KEY",
            VerifierError::MalformedJson { .. } => "MALFORMED_JSON",
            VerifierError::SchemaViolation { .. } => "SCHEMA_VIOLATION",
            VerifierError::DigestMismatch { .. } => "DIGEST_MISMATCH",
            VerifierError::SemanticError { .. } => "SEMANTIC_ERROR",
            VerifierError::SchemaIo(_) => "SCHEMA_IO_ERROR",
            VerifierError::OutputSchemaViolation { .. } => "OUTPUT_SCHEMA_VIOLATION",
            VerifierError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}
