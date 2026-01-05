fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=input.css");
    println!("cargo:rerun-if-changed=src/desktop/");
    println!("cargo:rerun-if-changed=src/mobile");

    let package_manager = install_dependencies();

    std::process::Command::new(package_manager)
        .args([
            "@tailwindcss/cli",
            "-i",
            "../../input.css",
            "-o",
            "./assets/tailwind.css",
            "--minify",
        ])
        .status()
        .expect("Failed to build UI assets");
}

fn install_dependencies() -> &'static str {
    let yarn = if cfg!(target_os = "windows") {
        "yarn.cmd"
    } else {
        "yarn"
    };

    let bunx = if cfg!(target_os = "windows") {
        "bunx.cmd"
    } else {
        "bunx"
    };

    let bun = if cfg!(target_os = "windows") {
        "bun.cmd"
    } else {
        "bun"
    };

    let npm = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };

    let npx = if cfg!(target_os = "windows") {
        "npx.cmd"
    } else {
        "npx"
    };

    if std::process::Command::new(bun)
        .arg("install")
        .spawn()
        .is_ok()
    {
        return bunx;
    };

    if std::process::Command::new(yarn)
        .arg("install")
        .spawn()
        .is_ok()
    {
        return yarn;
    };

    match std::process::Command::new(npm).arg("install").spawn() {
        Ok(_) => npx,
        Err(e) => panic!("ERROR: Npm, Bun or Yarn installation is needed.\n{e}"),
    }
}
