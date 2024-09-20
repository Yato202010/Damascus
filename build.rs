fn main() {
    println!("cargo:rerun-if-changed=vendor");

    #[cfg(feature = "fuse-overlayfs-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        let err = "Unable to Compile fuse-overlayfs";
        let executable = d.join("fuse-overlayfs");

        if !is_cache_valid(&executable) {
            run(err, d.join("autogen.sh"), &d);
            run(err, d.join("configure"), &d);
            run(err, "make", &d);
            cache(&executable);
        }
    }

    #[cfg(feature = "unionfs-fuse-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/unionfs-fuse/");
        if !d.exists() {
            panic!("unionfs-fuse submodule is not present unionfs-fuse-vendored cannot be used")
        }
        dbg!(&d);
        let build_d = d.join("build");
        if !build_d.exists() {
            std::fs::create_dir(&build_d).unwrap()
        }
        let executable = build_d.join("bin/unionfs");
        if !is_cache_valid(&executable) {
            use cmake::Config;
            let _ = Config::new(&d).out_dir(build_d).very_verbose(false).build();
            cache(&executable);
        }
    }
}

#[inline]
#[allow(dead_code)]
fn run<S: AsRef<std::ffi::OsStr> + std::fmt::Debug>(err: &str, op: S, dir: &std::path::PathBuf) {
    use std::process::Command;
    Command::new(&op)
        .current_dir(dir)
        .spawn()
        .unwrap_or_else(|_| panic!("{:?}: {:?} failed", err, op))
        .wait_with_output()
        .unwrap();
}

#[cfg(feature = "md5")]
const CACHE_DIR: &str = "target/.cache";

#[inline]
#[cfg(feature = "md5")]
fn is_cache_valid<S: AsRef<std::path::Path> + std::fmt::Debug>(path: S) -> bool {
    let path = path.as_ref();
    if !path.exists() {
        return false;
    }
    let cache = std::path::Path::new(CACHE_DIR).join(path.file_name().unwrap());
    if !cache.exists() {
        return false;
    }
    let old_hash = std::fs::read(cache).unwrap();
    let buf = std::fs::read(path).unwrap();
    let new_hash = md5::compute(&buf).0;
    old_hash == new_hash
}

#[inline]
#[cfg(feature = "md5")]
fn cache<S: AsRef<std::path::Path> + std::fmt::Debug>(path: S) {
    let path = path.as_ref();
    let buf = std::fs::read(path).unwrap();
    let hash = md5::compute(&buf).0;
    if !std::path::PathBuf::from(CACHE_DIR).exists() {
        std::fs::create_dir_all(CACHE_DIR).unwrap();
    }

    let cache = std::path::Path::new(CACHE_DIR).join(path.file_name().unwrap());
    std::fs::write(cache, hash).unwrap();
}
