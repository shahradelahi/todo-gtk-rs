use std::{env, path::Path, process::Command};

fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "resources.gresource",
    );

    compile_schemas(&["schemas"]);
}

// rustdoc-stripper-ignore-next
/// Call to run `glib-compile-schemas` to generate compiled gschemas from `.gschema.xml` schemas
/// files.
///
/// ```no_run
/// glib_build_tools::compile_schemas(
///     &["schemas"]
/// );
/// ```
pub fn compile_schemas(schemas_dir: &[&str]) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let target_dir = out_dir.join("gschemas");

    // Ensure target_dir exists
    std::fs::create_dir_all(&target_dir).expect("Failed to create target directory");

    println!(
        "cargo:rustc-env=GSETTINGS_SCHEMA_DIR={}",
        target_dir.to_str().unwrap()
    );

    // Recursively copy all files with .gschema.xml extension from schema_dir to target_dir
    for schema_dir in schemas_dir {
        let entries = Path::new(schema_dir)
            .read_dir()
            .expect("Failed to read schema directory")
            .flatten();

        for entry in entries {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();

            if path.is_file() && file_name.ends_with(".gschema.xml") {
                let target_path = target_dir.join(path.file_name().unwrap());
                std::fs::copy(&path, &target_path).expect("Failed to copy schema file");
            }
        }
    }

    let mut command = Command::new("glib-compile-schemas");
    command.arg("--strict");
    command.arg(target_dir);

    let output = command
        .output()
        .expect("Failed to execute glib-compile-schemas");

    assert!(
        output.status.success(),
        "glib-compile-schemas failed with exit status {} and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    for schema_dir in schemas_dir {
        println!("cargo:rerun-if-changed={}", schema_dir);
    }
}
