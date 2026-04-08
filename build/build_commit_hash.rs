use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let git_dir = manifest_dir.join("../../.git");
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    if let Ok(head) = fs::read_to_string(git_dir.join("HEAD")) {
        if let Some(reference) = head.strip_prefix("ref: ") {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference.trim()).display()
            );
        }
    }

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("failed to read git commit hash");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    let hash = String::from_utf8(output.stdout)
        .expect("git rev-parse HEAD should emit UTF-8")
        .trim()
        .to_owned();
    println!("cargo:rustc-env=BUILD_COMMIT_HASH={hash}");
}
