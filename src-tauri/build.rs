use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    setup_vosk();
    tauri_build::build()
}

fn setup_vosk() {
    let target = env::var("TARGET").unwrap_or_default();
    let out_dir = env::var("OUT_DIR").unwrap();
    let vosk_dir = PathBuf::from(&out_dir).join("vosk");

    let (url, lib_filename, zip_name, extracted_folder) = if target.contains("apple") || target.contains("darwin") {
        (
            "https://github.com/alphacep/vosk-api/releases/download/v0.3.42/vosk-osx-0.3.42.zip",
            "libvosk.dylib",
            "vosk-osx.zip",
            "vosk-osx-0.3.42",
        )
    } else if target.contains("windows") {
        (
            "https://github.com/alphacep/vosk-api/releases/download/v0.3.42/vosk-win64-0.3.42.zip",
            "libvosk.lib",
            "vosk-win64.zip",
            "vosk-win64-0.3.42",
        )
    } else if target.contains("linux") {
        let arch_suffix = if target.contains("aarch64") { "aarch64" } else { "x86_64" };
        let folder = format!("vosk-linux-{}-0.3.42", arch_suffix);
        // Leak strings to get 'static lifetime needed for the tuple
        let url_str = format!(
            "https://github.com/alphacep/vosk-api/releases/download/v0.3.42/vosk-linux-{}-0.3.42.zip",
            arch_suffix
        );
        (
            Box::leak(url_str.into_boxed_str()) as &str,
            "libvosk.so",
            "vosk-linux.zip",
            Box::leak(folder.into_boxed_str()) as &str,
        )
    } else {
        println!("cargo:warning=Vosk setup: unsupported target '{}', skipping", target);
        return;
    };

    let lib_path = vosk_dir.join(lib_filename);

    // Already downloaded - still need to ensure copies are in place
    if lib_path.exists() {
        copy_to_target_dirs(&lib_path, lib_filename, &target);
        ensure_libs_copy(&lib_path, lib_filename, &target);
        set_link_flags(&vosk_dir, &target);
        return;
    }

    println!("cargo:warning=Downloading Vosk library for {}...", target);

    if let Err(e) = download_and_extract_vosk(&vosk_dir, url, zip_name, extracted_folder, &target) {
        println!("cargo:warning=Vosk download failed: {}. Vosk engine will not be available.", e);
        return;
    }

    // macOS: fix install_name for runtime loading (via Frameworks dir)
    if target.contains("apple") || target.contains("darwin") {
        let _ = Command::new("install_name_tool")
            .args(["-id", "@rpath/libvosk.dylib", lib_path.to_str().unwrap()])
            .status();
    }

    // Copy to target directories for runtime access
    copy_to_target_dirs(&lib_path, lib_filename, &target);

    // macOS: copy to libs/ so Tauri can bundle it via frameworks config
    ensure_libs_copy(&lib_path, lib_filename, &target);

    println!("cargo:warning=Vosk library downloaded successfully");
    set_link_flags(&vosk_dir, &target);
}

fn download_and_extract_vosk(
    vosk_dir: &PathBuf,
    url: &str,
    zip_name: &str,
    extracted_folder: &str,
    target: &str,
) -> Result<(), String> {
    fs::create_dir_all(vosk_dir).map_err(|e| format!("Failed to create vosk directory: {}", e))?;

    let zip_path = vosk_dir.join(zip_name);

    // Download
    let download_tool = if target.contains("windows") { "curl.exe" } else { "curl" };
    let status = Command::new(download_tool)
        .args(["-L", "--fail", "--silent", "--show-error", "-o", zip_path.to_str().unwrap(), url])
        .status()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !status.success() {
        return Err("curl download failed".to_string());
    }

    // Extract
    if target.contains("windows") {
        // Use PowerShell on Windows
        let status = Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    zip_path.display(),
                    vosk_dir.display()
                ),
            ])
            .status()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if !status.success() {
            return Err("PowerShell extraction failed".to_string());
        }
    } else {
        let status = Command::new("unzip")
            .args(["-o", zip_path.to_str().unwrap(), "-d", vosk_dir.to_str().unwrap()])
            .status()
            .map_err(|e| format!("Failed to run unzip: {}", e))?;

        if !status.success() {
            return Err("unzip extraction failed".to_string());
        }
    }

    // Move files from extracted subfolder to vosk_dir
    let extracted_dir = vosk_dir.join(extracted_folder);
    if extracted_dir.exists() {
        for entry in fs::read_dir(&extracted_dir).map_err(|e| format!("Failed to read extracted dir: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let dest = vosk_dir.join(name);
                let _ = fs::rename(&path, &dest);
            }
        }
        fs::remove_dir_all(&extracted_dir).ok();
    }

    // Clean up zip
    fs::remove_file(&zip_path).ok();

    Ok(())
}

fn ensure_libs_copy(lib_path: &PathBuf, lib_filename: &str, target: &str) {
    if target.contains("apple") || target.contains("darwin") {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let libs_dir = PathBuf::from(&manifest_dir).join("libs");
        fs::create_dir_all(&libs_dir).ok();
        let dest = libs_dir.join(lib_filename);
        if !dest.exists() {
            fs::copy(lib_path, &dest).ok();
        }
    }
}

fn set_link_flags(vosk_dir: &PathBuf, target: &str) {
    println!("cargo:rustc-link-search=native={}", vosk_dir.display());
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=libvosk");
    } else {
        println!("cargo:rustc-link-lib=dylib=vosk");
    }
}

fn copy_to_target_dirs(lib_path: &PathBuf, lib_filename: &str, target: &str) {
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(d) => d,
        Err(_) => return,
    };

    let base = PathBuf::from(&manifest_dir).join("target");

    // Copy to both plain target dirs and target-triple-specific dirs (for cross-compilation)
    let mut dirs_to_copy = vec![
        base.join("debug"),
        base.join("release"),
    ];
    if !target.is_empty() {
        dirs_to_copy.push(base.join(target).join("debug"));
        dirs_to_copy.push(base.join(target).join("release"));
    }

    for dest_dir in dirs_to_copy {
        fs::create_dir_all(&dest_dir).ok();
        let dest_lib = dest_dir.join(lib_filename);
        if !dest_lib.exists() {
            if fs::copy(lib_path, &dest_lib).is_ok() {
                // macOS: fix install_name on the copied file too
                if target.contains("apple") || target.contains("darwin") {
                    let _ = Command::new("install_name_tool")
                        .args(["-id", "@rpath/libvosk.dylib", dest_lib.to_str().unwrap()])
                        .status();
                }
            }
        }
    }
}
