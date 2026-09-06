fn main() {
    slint_build::compile("ui/app-window.slint").expect("Failed to compile SLint UI");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.compile().expect("Failed to embed Windows icon");
    }
}
