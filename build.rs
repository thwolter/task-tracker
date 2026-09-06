fn main() {

    println!("cargo:rerun-if-changed=translations");

    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("translations");

    slint_build::compile_with_config("ui/app-window.slint", config)
        .expect("Failed to compile SLint UI");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icons/icon.ico");
        res.compile().expect("Failed to embed Windows icon");
    }
}
