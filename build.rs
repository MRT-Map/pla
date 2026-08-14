fn main() -> Result<(), std::io::Error> {
    const EMATH_HIGHEST_VERSION: &str = "emath036";
    const BEZIER_EPAINT_HIGHEST_VERSION: &str = "epaint036";

    let glam_file = std::fs::read_to_string(format!("src/node_type/{EMATH_HIGHEST_VERSION}.rs"))?;
    let replace_from = format!("use {EMATH_HIGHEST_VERSION} as emath;");
    for target in [
        "emath035", "emath034", "emath033", "emath032", "emath031", "emath030", "emath029",
        "emath028",
    ] {
        std::fs::write(
            format!("src/node_type/{target}.rs"),
            glam_file.replace(&replace_from, &format!("use {target} as emath;")),
        )?;
    }
    println!("cargo:rerun-if-changed=src/node_type/{EMATH_HIGHEST_VERSION}.rs");

    let glam_file =
        std::fs::read_to_string(format!("src/node_type/{BEZIER_EPAINT_HIGHEST_VERSION}.rs"))?;
    let replace_from = format!("use {BEZIER_EPAINT_HIGHEST_VERSION} as epaint;");
    for target in [
        "epaint035",
        "epaint034",
        "epaint033",
        "epaint032",
        "epaint031",
        "epaint030",
        "epaint029",
        "epaint028",
    ] {
        std::fs::write(
            format!("src/node_type/{target}.rs"),
            glam_file.replace(&replace_from, &format!("use {target} as epaint;")),
        )?;
    }
    println!("cargo:rerun-if-changed=src/node_type/{BEZIER_EPAINT_HIGHEST_VERSION}.rs");

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
