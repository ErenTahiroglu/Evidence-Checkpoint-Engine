// mod.rs — GitHub read-only evidence adapter module root.

pub mod client;
pub mod evaluator;
pub mod models;
pub mod policy;

pub use client::{GitHubClientError, GitHubProvider, HttpGitHubClient, MockGitHubProvider};
pub use evaluator::{evaluate_github_evidence, DeclaredArtifactSpec, TargetEvidenceRequest};
pub use models::*;
pub use policy::{CheckPolicyEvaluation, PolicyStatus, TrustedCheckPolicy};
