use gix::Repository;
use semver::{Version, VersionReq};
use anyhow::Result;

pub enum ResolvedVersion {
    Tag(String),
    Branch(String),
    Commit(String),
}

pub async fn resolve_version(package: &Package) -> Result<ResolvedVersion> {
    match package.version.as_str() {
        "latest" => resolve_latest(&package.source).await,
        v if v.starts_with('^') || v.starts_with('~') => {
            resolve_semver_range(&package.source, v).await
        }
        v => Ok(ResolvedVersion::Explicit(v.to_string())),
    }
}

async fn resolve_latest(repo_url: &str) -> Result<ResolvedVersion> {
    // 1. Try to find latest semver tag
    if let Some(tag) = find_latest_semver_tag(repo_url).await? {
        return Ok(ResolvedVersion::Tag(tag));
    }
    
    // 2. Fallback to default branch
    let branch = find_default_branch(repo_url).await?;
    Ok(ResolvedVersion::Branch(branch))
}
