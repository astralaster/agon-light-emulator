fn get_git_hash() -> String {
    let output = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to execute git command.");

    let git_hash = String::from_utf8(output.stdout).expect("Failed to convert git output to string.");

    git_hash
}

fn main() {
    let git_hash = get_git_hash();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}