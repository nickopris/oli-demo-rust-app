//! OLI copies this whole repo into the user's Drupal project, and the repo is public.
//! These checks keep what lands there correct and free of local details.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

/// The HTTPS router port the host hook publishes.
fn hook_https_port() -> String {
    let hook = read(".drupalaibp/rust-demo-ddev.sh");
    hook.lines()
        .find_map(|l| l.trim().strip_prefix("https_port:"))
        .map(|p| p.trim().to_string())
        .expect("hook sets https_port")
}

#[test]
fn launch_path_and_welcome_use_the_published_port() {
    let port = hook_https_port();
    let cfg: serde_json::Value = serde_json::from_str(&read(".drupalaibp/config.json")).unwrap();
    assert_eq!(cfg["launch_path"], format!(":{port}"));
    assert!(cfg["append_welcome_text"][0]
        .as_str()
        .unwrap()
        .contains(&format!(":{port}")));
    assert!(read("README.md").contains(&format!(":{port}")));
}

#[test]
fn router_ports_avoid_common_dev_ports() {
    let hook = read(".drupalaibp/rust-demo-ddev.sh");
    for busy in ["8080", "8443", "8000"] {
        assert!(
            !hook.contains(&format!("http_port: {busy}"))
                && !hook.contains(&format!("https_port: {busy}")),
            "router port {busy} is commonly taken on dev machines"
        );
    }
}

#[test]
fn gitignore_suits_the_drupal_project_it_is_copied_into() {
    let gi = read(".gitignore");
    let lines: Vec<&str> = gi.lines().map(str::trim).collect();
    assert!(
        !lines
            .iter()
            .any(|l| l.trim_start_matches('/').starts_with(".claude")),
        "OLI installs .claude/ in the project"
    );
    for needed in [
        "/vendor/",
        "/web/core/",
        "/target/",
        ".env*",
        ".ddev/rust-demo/bin/",
    ] {
        assert!(lines.contains(&needed), "missing {needed}");
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if p.is_dir() {
            if !matches!(
                name.as_ref(),
                "target" | ".git" | ".superpowers" | ".claude" | ".idea"
            ) {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

#[test]
fn no_local_paths_or_ide_files_in_the_repo() {
    assert!(
        !root().join(".idea").exists() || !tracked(".idea"),
        ".idea/ must not be committed"
    );
    // Built at runtime so this file doesn't match itself.
    let home = ["/", "Users", "/"].concat();
    let mut files = Vec::new();
    walk(&root(), &mut files);
    for f in files {
        if let Ok(text) = std::fs::read_to_string(&f) {
            assert!(
                !text.contains(&home),
                "absolute local path in {}",
                f.display()
            );
        }
    }
}

/// True when git tracks something under `rel` (git may be missing in some sandboxes).
fn tracked(rel: &str) -> bool {
    std::process::Command::new("git")
        .args(["ls-files", rel])
        .current_dir(root())
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

// Drupal CMS ships without the core config module; setup-tips.sh needs it
// for `drush config:import --partial`.
#[test]
fn install_modules_enable_what_setup_tips_needs() {
    let cfg: serde_json::Value = serde_json::from_str(&read(".drupalaibp/config.json")).unwrap();
    let modules: Vec<&str> = cfg["install_modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert!(read(".drupalaibp/setup-tips.sh").contains("config:import --partial"));
    for needed in ["jsonapi", "config"] {
        assert!(modules.contains(&needed), "install_modules lacks {needed}");
    }
}
