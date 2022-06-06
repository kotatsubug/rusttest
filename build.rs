extern crate walkdir;

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Locate .exe path even if the project is in workspace
    let executable_path = locate_target_dir_from_output_dir(&out_dir)
        .expect("failed to find target dir")
        .join(std::env::var("PROFILE")
        .unwrap()
    );

    copy(
        &manifest_dir.join("assets"),
        &executable_path.join("assets"),
    );
}

fn locate_target_dir_from_output_dir(mut target_dir_search: &std::path::Path) -> Option<&std::path::Path> {
    loop {
        // If the path ends with "target", assume this is correct directory
        if target_dir_search.ends_with("target") {
            return Some(target_dir_search);
        }
        // Otherwise, keep going up in tree until "target" directory is found
        target_dir_search = match target_dir_search.parent() {
            Some(path) => path,
            None => break,
        }
    }

    None
}

fn copy(from: &std::path::Path, to: &std::path::Path) {
    let from_path: std::path::PathBuf = from.into();
    let to_path: std::path::PathBuf = to.into();

    for entry in walkdir::WalkDir::new(from_path.clone()) {
        let entry = entry.unwrap();

        if let Ok(rel_path) = entry.path().strip_prefix(&from_path) {
            let target_path = to_path.join(rel_path);
            
            if entry.file_type().is_dir() {
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .create(target_path)
                    .expect("failed to create target dir");
            } else {
                std::fs::copy(entry.path(), &target_path).expect("failed to copy");
            }
        }
    }
}