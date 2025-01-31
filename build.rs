// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
fn main() {
    println!("cargo:rerun-if-changed=vendor");

    #[cfg(all(feature = "fuse-overlayfs-vendored", target_os = "linux"))]
    {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/fuse-overlayfs/");
        if !d.exists() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        const EXEC: &str = "fuse-overlayfs";
        let basedir = std::path::Path::new("./target/vendored");
        let outdir = basedir.join(EXEC);
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir).unwrap()
        }
        let exec_path = outdir.join("bin").join(EXEC);
        let build = || {
            let srcdir = outdir.join("src");
            if !srcdir.exists() {
                std::fs::create_dir(&srcdir).unwrap();
            }
            std::fs::remove_dir_all(&srcdir).unwrap();
            fs_extra::dir::copy(
                &d,
                &srcdir,
                &fs_extra::dir::CopyOptions::new()
                    .overwrite(true)
                    .content_only(true)
                    .copy_inside(true)
                    .skip_exist(false),
            )
            .unwrap();
            autotools::Config::new(srcdir)
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

    #[cfg(all(feature = "unionfs-fuse-vendored", target_os = "linux"))]
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
    #[allow(dead_code)]
    #[cfg(feature = "build-cache")]
    pub fn need_rebuild<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
        src: P,
        out: Q,
        exception: Vec<String>,
    ) -> bool {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct Cache {
            new_or_edited: Vec<(String, [u8; 16])>,
        }

        let cache_file = out.as_ref().join("cache.tag");
        let cache: Option<Cache> = std::fs::File::open(&cache_file)
            .ok()
            .map(std::io::BufReader::new)
            .and_then(|x| serde_json::from_reader(x).ok());

        let mut hashed = vec![];

        fn check_dir(
            hashed: &mut Vec<(String, [u8; 16])>,
            src: &std::path::Path,
            exception: &[String],
        ) {
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    check_dir(hashed, &path, exception);
                    continue;
                }
                let path_str = path.to_string_lossy().to_string();
                if !exception.contains(&path_str) {
                    let buf = std::fs::read(src.join(&path_str)).unwrap();
                    let hash = md5::compute(&buf).0;
                    hashed.push((path_str, hash));
                }
            }
        }

        check_dir(&mut hashed, src.as_ref(), &exception);

        if cache.is_none_or(|x| x.new_or_edited != hashed) {
            println!("Detected changes in vendored dependency. Updating cache.");

            let new_cache = Cache {
                new_or_edited: hashed,
            };
            std::fs::write(cache_file, serde_json::to_string(&new_cache).unwrap())
                .expect("Unable to write cache file");
            true
        } else {
            println!("No changes detected in vendored dependency.");
            false
        }
    }
}
