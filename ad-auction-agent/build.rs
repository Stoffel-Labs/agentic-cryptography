use std::path::PathBuf;

// Generate typed Stoffel client IO bindings from the auction source.
// (Development mode: bindings follow the committed .stfl. Rebuild bytecode and
// regenerate after any source change before a deployment-shaped release.)
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/auction.stfl");

    let out_file = PathBuf::from(std::env::var("OUT_DIR")?).join("stoffel_bindings.rs");
    stoffel_bindgen::generate_bindings_from_source(
        "src/auction.stfl",
        out_file,
        stoffel_bindgen::BindingsConfig::default(),
    )?;

    Ok(())
}
