fn main() {
    // Le manifeste UAC (requireAdministrator) doit être embarqué PAR tauri-build
    // lui-même. Trois pièges successifs documentés au bug #29:
    //
    // 1. embed-resource + app.rc avec `let _ =` étaient écrasés par le manifeste
    //    par défaut de tauri-build (Common-Controls v6 seul, aucune
    //    requestedExecutionLevel): l'exe démarrait en asInvoker, sans invite UAC.
    // 2. WindowsAttributes::app_manifest plante le manifeste dans un bloc de
    //    chaînes rc (tauri-winres écrit `" {ligne} "` par ligne, concaténées):
    //    déclaration `<?xml` transformée en ` <?xml`, accents perdus, et surtout
    //    CreateActCtx rejette le résultat en erreur SxS 14001.
    // 3. Solution retenue: inclure le fichier TEL QUEL via append_rc_content
    //    (`1 24 "chemin"`), comme set_manifest_file de tauri-winres — rc.exe
    //    embarque les octets du fichier sans aucune transformation.
    //
    // Seuls les builds release portent la demande d'élévation: un binaire de
    // test (cargo test) ou de dev compilé avec requireAdministrator refuse de
    // démarrer dans un shell non élevé (os error 14001 / Permission denied).
    let windows = if std::env::var("PROFILE").is_ok_and(|p| p == "release") {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("app.manifest");
        // Séparateurs avant: rc.exe inclut le chemin tel quel.
        let manifest = manifest.to_string_lossy().replace('\\', "/");
        tauri_build::WindowsAttributes::new_without_app_manifest()
            .append_rc_content(format!("1 24 \"{manifest}\""))
    } else {
        // Debug/test: manifeste par défaut de tauri-build (Common-Controls v6).
        tauri_build::WindowsAttributes::new()
    };

    // Sans cela, cargo ne recompile pas quand seul le manifeste change
    // (append_rc_content ne déclare aucune dépendance).
    println!("cargo:rerun-if-changed=app.manifest");

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri_build::try_build a échoué (manifeste Windows)");
}
