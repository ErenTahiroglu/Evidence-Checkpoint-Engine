// verifier/mod.rs — Public verification library API.

pub mod assessment;
pub mod canonicalize;
pub mod digest;
pub mod error;
pub mod parser;
pub mod schema;
pub mod semantic;

pub use assessment::{
    build_assessment_digest_fail, build_assessment_inspect_untrusted, build_assessment_limitations,
    build_assessment_refused, inspect_payload, verify_envelope, VerificationOutput,
};
pub use canonicalize::canonicalize;
pub use digest::{compute_digest, verify_envelope_digest};
pub use error::{ErrorCode, VerifierError};
pub use parser::{parse_bounded_json, MAX_INPUT_BYTES, MAX_NESTING_DEPTH};
pub use schema::{default_schema_dir, SchemaStore};
pub use semantic::{run_semantic_checks, SemanticResult};
