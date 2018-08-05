use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
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

    // Generating build globals
    let default_locales = "./podcasts-gtk/po".to_string();
    let out_dir = env::var("OUT_DIR").unwrap();
    let localedir = env::var("PODCASTS_LOCALEDIR").unwrap_or(default_locales);
    let dest_path = Path::new(&out_dir).join("build_globals.rs");
    let mut f = File::create(&dest_path).unwrap();

    let globals = format!(
        "
pub(crate) static LOCALEDIR: &'static str = \"{}\";
",
        localedir
    );

    f.write_all(&globals.into_bytes()[..]).unwrap();
}
