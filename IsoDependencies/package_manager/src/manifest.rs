use anyhow::{Context, Result};
use regex::Regex;

/// Detect current architecture string for placeholder replacement.
pub fn infer_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "riscv64" => "riscv64",
        other => other,
    }
}

/// Replace `{tag}` and `{arch}` placeholders in URLs.
pub fn apply_placeholders(s: &str, tag: &str) -> String {
    s.replace("{tag}", tag).replace("{arch}", infer_arch())
}

/// Given a list of git tag strings, pick the highest semantic version.
pub fn pick_latest_tag(tags: &[String]) -> Option<String> {
    let re = Regex::new(r"^v?(\\d+)\\.(\\d+)\\.(\\d+)$").unwrap();
    let mut vs: Vec<(u32, u32, u32, String)> = tags
        .iter()
        .filter_map(|t| {
            re.captures(t).map(|c| {
                (
                    c[1].parse().ok()?,
                    c[2].parse().ok()?,
                    c[3].parse().ok()?,
                    t.clone(),
                )
            })
        })
        .collect();
    vs.sort();
    vs.last().map(|(_, _, _, s)| s.clone())
}

/// Use `git ls-remote --tags` to find the latest semver tag.
pub fn resolve_latest_git_tag(repo_url: &str) -> Result<String> {
    use std::process::Command;
    let out = Command::new("git")
        .arg("ls-remote")
        .arg("--tags")
        .arg(repo_url)
        .output()
        .with_context(|| format!("running git ls-remote on {}", repo_url))?;
    anyhow::ensure!(out.status.success(), "git ls-remote failed");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut tags = Vec::new();
    for line in stdout.lines() {
        if let Some(pos) = line.rsplit('/').next() {
            tags.push(pos.to_string());
        }
    }

    pick_latest_tag(&tags).context("no semver tags found")
}
