fn get_git_hash() -> String {
    let output_tag = std::process::Command::new("git").args(&["tag", "--points-at", "HEAD"]).output();
    
    if output_tag.is_ok() {
        let tag = output_tag.unwrap();
        if tag.status.success() {
            let git_tag = String::from_utf8(tag.stdout).expect("Failed to convert git output to string.");
            if git_tag.len() > 0 {
                return git_tag;
            }
        }
    }    
    
    let output_hash = std::process::Command::new("git").args(&["rev-parse", "--short", "HEAD"]).output();
    if output_hash.is_ok() {
        let hash = output_hash.unwrap();
        if hash.status.success() {
            let git_hash = String::from_utf8(hash.stdout).expect("Failed to convert git output to string.");
            if git_hash.len() > 0 {
                return git_hash;
            }
        }
    }
    
    return String::from("unknown"); // Failed to get
}

fn main() {
    let git_hash = get_git_hash();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}