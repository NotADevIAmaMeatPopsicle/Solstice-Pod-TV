fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Pod Patcher");
        res.set("FileDescription", "Pod Patcher - Solstice Pod Gen3 Flasher and Manager");
        let _ = res.compile();
    }
}
