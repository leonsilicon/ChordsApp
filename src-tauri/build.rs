fn main() {
    println!("cargo:rerun-if-changed=native/hid.c");
    println!("cargo:rerun-if-changed=native/caps.c");
    println!("cargo:rerun-if-changed=data/generated/macos/chords.json");

    cc::Build::new()
        .file("native/hid.c")
        .file("native/caps.c")
        .compile("hid_caps");

    tauri_build::build()
}
