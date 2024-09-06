use std::{ffi::OsStr, fmt::Debug, path::PathBuf};

fn main() {
    #[cfg(feature = "fuse-overlayfs-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        let err = "Unable to Compile fuse-overlayfs";
        let mut op = "autogen.sh";
        run(err, d.join(op), &d);
        op = "configure";
        run(err, d.join(op), &d);
        op = "make";
        run(err, op, &d)
    }
    #[cfg(feature = "cicpoffs-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/cicpoffs/");
        if !d.exists() {
            panic!("cicpoffs submodule is not present cicpoffs-vendored cannot be used")
        }
        run("Unable to Compile cicpoffs", "make", &d)
    }
}

#[inline]
#[allow(dead_code)]
fn run<S: AsRef<OsStr> + Debug>(err: &str, op: S, dir: &PathBuf) {
    use std::process::Command;
    Command::new(&op)
        .current_dir(dir)
        .spawn()
        .unwrap_or_else(|_| panic!("{:?}: {:?} failed", err, op));
}
