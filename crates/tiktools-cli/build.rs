use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TIKTOOLS_BUILD_SHA");
    let revision = std::env::var("TIKTOOLS_BUILD_SHA").ok().or_else(|| {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    });
    if let Some(revision) =
        revision.filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        println!("cargo:rustc-env=TIKTOOLS_BUILD_SHA={revision}");
    }
    let mut refs = vec!["HEAD".to_owned(), "packed-refs".to_owned()];
    if let Ok(output) = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
    {
        if output.status.success() {
            refs.push(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
    }
    for reference in refs {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--git-path", &reference])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                let path = path.trim();
                if std::path::Path::new(path).exists() {
                    println!("cargo:rerun-if-changed={path}");
                }
            }
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
