fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        res.set("ProductName", "Hasher");
        res.set("FileDescription", "Hasher — calcolo e verifica hash");
        res.set("CompanyName", "Martini Multimedia s.a.s.");
        res.set("LegalCopyright", "© Martini Multimedia s.a.s.");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winresource failed: {e}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
}
