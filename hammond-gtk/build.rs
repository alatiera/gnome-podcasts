use std::process::Command;

fn main() {
    // Rerun the build script when files in the resources folder are changed.
    println!("cargo:rerun-if-changed=resources");
    println!("cargo:rerun-if-changed=resources/*");

    Command::new("glib-compile-resources")
        .args(&["--generate", "resources.xml"])
        .current_dir("resources")
        .status()
        .unwrap();
}
