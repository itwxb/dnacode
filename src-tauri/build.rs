fn main() {
    // 图标变化时强制重新运行构建脚本（否则 Cargo 可能复用旧资源库，
    // 导致本地增量构建的 exe 仍嵌着旧图标；CI 干净构建不受影响）。
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
