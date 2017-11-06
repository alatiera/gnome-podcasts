use std::process::Command;

fn main() {
    Command::new("glib-compile-resources")
        .args(&["--generate", "resources.xml"])
        .current_dir("src/resources")
        .status()
        .unwrap();
}
