use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=public");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/packed-refs");

    // Get short git hash for current HEAD.
    let git_hash = git_output(&["rev-parse", "--short=8", "HEAD"]);
    // Get commit timestamp (ISO-8601) for tooltip display.
    let git_timestamp = git_output(&["show", "-s", "--format=%cI", "HEAD"]);

    if let Some(git_hash) = git_hash {
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    }
    if let Some(git_timestamp) = git_timestamp {
        println!("cargo:rustc-env=GIT_TIMESTAMP={}", git_timestamp);
    }

    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let sw_path = Path::new("public/sw.js");

    if !sw_path.exists() {
        return;
    }

    let mut content = fs::read_to_string(sw_path).expect("Failed to read sw.js");

    // Update CACHE_NAME based on Cargo version
    let cache_def = format!("const CACHE_NAME = 'hocg-deck-convert-v{}';", version);
    if let Some(start) = content.find("const CACHE_NAME =")
        && let Some(end) = content[start..].find(";")
    {
        let actual_end = start + end + 1;
        content.replace_range(start..actual_end, &cache_def);
    }

    // Generate ASSETS list from public/ and index.html
    let public_dir = Path::new("public");
    let mut files = Vec::new();

    files.push("BASE_PATH + '/'".to_string());
    files.push("BASE_PATH + '/index.html'".to_string());

    collect_files(public_dir, public_dir, &mut files);

    let mut assets_block = String::from("const ASSETS = [\n");
    for (i, f) in files.iter().enumerate() {
        assets_block.push_str(&format!("  {}", f));
        if i < files.len() - 1 {
            assets_block.push(',');
        }
        assets_block.push('\n');
    }
    assets_block.push_str("];");

    // Inject updated ASSETS list
    if let Some(start) = content.find("const ASSETS = [")
        && let Some(end) = content[start..].find("];")
    {
        let actual_end = start + end + 2;
        content.replace_range(start..actual_end, &assets_block);

        // Only write to disk if content actually changed to preserve mtime
        let on_disk = fs::read_to_string(sw_path).unwrap();
        if on_disk != content {
            fs::write(sw_path, content).expect("Failed to write sw.js");
        }
    }
}

fn collect_files(dir: &Path, base: &Path, files: &mut Vec<String>) {
    // Recursively walk directory
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, base, files);
            } else {
                let name = path.file_name().unwrap().to_str().unwrap();
                // Filter out sw.js, loader, and dotfiles
                if name == "sw.js" || name == "pwa-loader.js" || name.starts_with(".") {
                    continue;
                }
                let rel_path = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace("\\", "/");
                files.push(format!("BASE_PATH + '/{}'", rel_path));
            }
        }
    }
}
