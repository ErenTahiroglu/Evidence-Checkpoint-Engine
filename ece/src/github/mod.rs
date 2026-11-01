// mod.rs — GitHub read-only evidence adapter module root.

pub mod client;
pub mod evaluator;
#[cfg(feature = "github-http")]
pub mod http_client;
pub mod models;
pub mod policy;

#[cfg(feature = "github-http")]
pub use client::HttpGitHubClient;
pub use client::{GitHubClientError, GitHubProvider, MockGitHubProvider};
pub use evaluator::{
    evaluate_github_evidence, evaluate_github_evidence_with_store, DeclaredArtifactSpec,
    EnvelopeVerificationContext, TargetEvidenceRequest,
};
pub use models::*;
pub use policy::{CheckPolicyEvaluation, PolicyStatus, TrustedCheckPolicy};
