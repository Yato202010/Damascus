fn main() {
    println!("cargo:rerun-if-changed=vendor");

    #[cfg(any(feature = "fuse-overlayfs-vendored", feature = "unionfs-fuse-vendored"))]
    vendored::init_submodule();

    #[cfg(feature = "fuse-overlayfs-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        const EXEC: &str = "fuse-overlayfs";
        let outdir = std::path::Path::new("./target/vendored").join(EXEC);
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir).unwrap()
        }
        let exec_path = outdir.join("bin").join(EXEC);
        let build = || {
            autotools::Config::new(&d)
                .out_dir(std::fs::canonicalize(&outdir).expect("cannot canonicalize"))
                .reconf("-fis")
                .build()
        };
        #[cfg(feature = "build-cache")]
        if vendored::need_rebuild(&d, &outdir, vec![]) {
            build();
        }
        #[cfg(not(feature = "build-cache"))]
        build();
        println!(
            "cargo::rustc-env=FUSE-OVERLAYFS-BIN={}",
            &exec_path.to_string_lossy()
        );
    }

    #[cfg(feature = "unionfs-fuse-vendored")]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/unionfs-fuse/");
        if !d.exists() {
            panic!("unionfs-fuse submodule is not present unionfs-fuse-vendored cannot be used")
        }

        const EXEC: &str = "unionfs";
        let outdir = std::path::Path::new("./target/vendored").join(EXEC);
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir).unwrap()
        }
        let exec_path = outdir.join("build/src").join(EXEC);
        let build = || {
            cmake::Config::new(&d)
                .out_dir(std::fs::canonicalize(&outdir).expect("cannot canonicalize"))
                .very_verbose(false)
                .build();
        };
        #[cfg(feature = "build-cache")]
        if vendored::need_rebuild(&d, &outdir, vec![]) {
            build();
        }
        #[cfg(not(feature = "build-cache"))]
        build();
        println!(
            "cargo::rustc-env=UNIONFS-FUSE-BIN={}",
            &exec_path.to_string_lossy()
        );
    }
}

#[cfg(any(feature = "fuse-overlayfs-vendored", feature = "unionfs-fuse-vendored"))]
mod vendored {
    #[inline]
    pub fn init_submodule() {
        let repo = git::Repository::open(".").unwrap();
        let submodules = repo.submodules().unwrap();
        for mut submodule in submodules {
            let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(submodule.path())
                .unwrap()
                .map(|x| x.unwrap())
                .collect();
            if entries.is_empty() {
                submodule.update(true, None).unwrap();
            }
        }
    }

    #[inline]
    #[cfg(feature = "build-cache")]
    pub fn need_rebuild<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
        src: P,
        out: Q,
        exception: Vec<String>,
    ) -> bool {
        use git::{Repository, StatusOptions};
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct Cache {
            commit: String,
            new_or_edited: Vec<(String, [u8; 16])>,
        }

        let cache_file = out.as_ref().join("cache.tag");
        let cache: Option<Cache> = std::fs::File::open(&cache_file)
            .ok()
            .map(std::io::BufReader::new)
            .and_then(|x| serde_json::from_reader(x).ok());

        if let Ok(repo) = Repository::open(&src) {
            let latest_commit_hash = repo
                .head()
                .expect("No HEAD")
                .target()
                .expect("No target")
                .to_string();
            let mut hashed = vec![];

            for elem in repo
                .statuses(Some(StatusOptions::new().include_untracked(true)))
                .unwrap()
                .iter()
            {
                let path_str = elem.path().expect("not a valid utf8").to_string();
                dbg!(&path_str);
                if !exception.contains(&path_str) {
                    let buf = std::fs::read(src.as_ref().join(&path_str)).unwrap();
                    let hash = md5::compute(&buf).0;
                    hashed.push((path_str.clone(), hash));
                }
            }

            // Write the latest commit hash to a cache file if it's different from the cached one
            return if !cache
                .is_some_and(|x| x.commit == latest_commit_hash && x.new_or_edited == hashed)
            {
                println!("Detected changes in vendored dependency. Updating cache.");

                let new_cache = Cache {
                    commit: latest_commit_hash,
                    new_or_edited: hashed,
                };
                // Update the cache file with the latest commit hash
                std::fs::write(cache_file, serde_json::to_string(&new_cache).unwrap())
                    .expect("Unable to write cache file");
                true
            } else {
                println!("No changes detected in vendored dependency.");
                false
            };
        }
        true
    }
}
