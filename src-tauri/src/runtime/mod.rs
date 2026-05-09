pub mod deps;
pub mod downloader;
pub mod locator;
pub mod packages;

// Re-exports
pub use deps::{Dependency, DependencyCheckResult, InstallCommand};
