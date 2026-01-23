fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../input.css");
    println!("cargo:rerun-if-changed=/src/desktop/");
    println!("cargo:rerun-if-changed=/src/mobile");
    println!("cargo:rerun-if-changed=../../package.json");

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
    let yarn = "yarn";

    let bunx = "bunx";

    let bun = "bun";

    let npm = "npm";

    let npx = "npx";

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
