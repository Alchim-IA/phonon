fn main() {
    // Configuration pour trouver les libs OpenVINO au build
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let openvino_lib_path = format!("{}/resources/openvino", manifest_dir);

    // Ajouter le chemin de recherche pour le linker
    println!("cargo:rustc-link-search=native={}", openvino_lib_path);

    // Configurer DYLD_LIBRARY_PATH pour le runtime (macOS)
    println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}", openvino_lib_path);

    tauri_build::build()
}
