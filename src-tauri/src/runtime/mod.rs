pub mod deps;
pub mod downloader;
pub mod locator;
pub mod packages;

// Re-exports
pub use deps::{Dependency, DependencyCheckResult, InstallCommand};

// Runtime binary download and location
// TODO: Implement in Phase 2
