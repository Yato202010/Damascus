use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::create_dir_all,
    path::{self, Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=vendor");

    #[cfg(feature = "fuse-overlayfs-vendored")]
    {
        // cache executable by storing it's hash in the target directory
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        let err = "Unable to Compile fuse-overlayfs";
        let executable = d.join("fuse-overlayfs");

        if !is_cache_valid(&executable) {
            let mut op = "autogen.sh";
            run(err, d.join(op), &d);
            op = "configure";
            run(err, d.join(op), &d);
            op = "make";
            run(err, op, &d);
            cache(&executable);
        }
    }
}

#[inline]
#[allow(dead_code)]
fn run<S: AsRef<OsStr> + Debug>(err: &str, op: S, dir: &PathBuf) {
    use std::process::Command;
    Command::new(&op)
        .current_dir(dir)
        .spawn()
        .unwrap_or_else(|_| panic!("{:?}: {:?} failed", err, op))
        .wait()
        .unwrap();
}

const CACHE_DIR: &str = "target/.cache";

#[inline]
#[allow(dead_code)]
fn is_cache_valid<S: AsRef<Path> + Debug>(path: S) -> bool {
    let path = path.as_ref();
    if !path.exists() {
        return false;
    }
    let cache = Path::new(CACHE_DIR).join(path.file_name().unwrap());
    if !cache.exists() {
        return false;
    }
    let old_hash = std::fs::read(cache).unwrap();
    let buf = std::fs::read(path).unwrap();
    let new_hash = md5::compute(&buf).0;
    old_hash == new_hash
}

#[inline]
#[allow(dead_code)]
fn cache<S: AsRef<Path> + Debug>(path: S) {
    let path = path.as_ref();
    let buf = std::fs::read(path).unwrap();
    let hash = md5::compute(&buf).0;
    if !PathBuf::from(CACHE_DIR).exists() {
        create_dir_all(CACHE_DIR).unwrap();
    }

    let cache = Path::new(CACHE_DIR).join(path.file_name().unwrap());
    std::fs::write(cache, hash).unwrap();
}
