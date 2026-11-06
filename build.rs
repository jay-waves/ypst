use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output.status.success().then(|| {
        String::from_utf8(output.stdout)
            .expect("git output must be UTF-8")
            .trim()
            .to_owned()
    })
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=YPST_BUILD_COMMIT");
    for name in ["HEAD", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&["rev-parse", "--git-path", &reference])
    {
        println!("cargo:rerun-if-changed={path}");
    }
    let commit = std::env::var("YPST_BUILD_COMMIT")
        .ok()
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .expect("build from a Git checkout or set YPST_BUILD_COMMIT");
    assert!(
        matches!(commit.len(), 40 | 64) && commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "YPST_BUILD_COMMIT must be a full Git commit ID"
    );
    println!("cargo:rustc-env=YPST_BUILD_COMMIT={commit}");
}
