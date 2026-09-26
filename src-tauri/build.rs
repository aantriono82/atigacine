fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/atiga-cine/lib");

    tauri_build::build();
}
