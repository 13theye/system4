use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get the output directory from cargo
    let out_dir = env::var("OUT_DIR").unwrap();
    let _profile = env::var("PROFILE").unwrap();

    // Copy config.toml to the build output directory
    let config_path = Path::new("config.toml");
    let assets = Path::new("assets");

    let top_path = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let mut desired_path_string = top_path.to_str().unwrap().to_owned();
    desired_path_string.push_str("/system4-support");

    let mut new_assets_path_string = desired_path_string.clone();
    new_assets_path_string.push_str("/assets");

    // Create support subdirectory for this project
    fs::create_dir_all(&desired_path_string).unwrap();

    // Copy config file
    let new_config_path = Path::new(&desired_path_string).join("config.toml");
    fs::copy(config_path, new_config_path).unwrap();

    // Copy assets
    let new_assets_path = Path::new(&new_assets_path_string);
    copy_dir_all(assets, new_assets_path).unwrap();
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
