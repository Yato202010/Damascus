fn main() {
    #[cfg(feature = "fuse-overlayfs-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        use std::process::Command;
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        Command::new(d.join("autogen.sh"))
            .current_dir(&d)
            .spawn()
            .expect("Unable to Compile fuse-overlayfs");
        Command::new(d.join("configure"))
            .current_dir(&d)
            .spawn()
            .expect("Unable to Compile fuse-overlayfs");
        Command::new("make")
            .current_dir(&d)
            .spawn()
            .expect("Unable to Compile fuse-overlayfs");
    }
}
