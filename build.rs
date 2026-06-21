fn main() {
    // 预编译的 .res 文件包含应用图标（从 icon.ico 生成）。
    // app.res 由 Windows rc.exe 预编译，规避 Rust 1.96.0 子进程 bug。
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    println!("cargo:rerun-if-changed=app.res");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let res_path = std::path::Path::new(&manifest_dir).join("app.res");
    // MSVC rustc 将 .res 文件直接转发给链接器
    let res_arg = res_path.display().to_string().replace('\\', "/");
    println!("cargo:rustc-link-arg={}", res_arg);
}
