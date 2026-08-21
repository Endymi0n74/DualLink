fn main() {
    tauri_build::build();

    // Embed the UAC manifest for admin auto-elevation
    #[cfg(windows)]
    {
        let _ = embed_resource::compile("app.rc", &[env!("CARGO_MANIFEST_DIR")]);
    }
}
