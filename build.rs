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
        if !d.exists() && vendored::download_submodule().is_err() {
            panic!("fuse-overlayfs submodule is not present fuse-overlayfs-vendored cannot be used")
        }
        const EXEC: &str = "fuse-overlayfs";
        let basedir = std::path::Path::new("./target/vendored");
        let outdir = basedir.join(EXEC);
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir)
                .expect("failed to create output directory for fuse-overlayfs")
        }
        let exec_path = outdir.join(EXEC);
        let build = || {
            let outdir_abs = std::fs::canonicalize(&outdir)
                .expect("cannot canonicalize outdir for fuse-overlayfs");
            let status = std::process::Command::new("cargo")
                .args([
                    "build",
                    "--release",
                    "--target-dir",
                    outdir_abs.to_str().expect("outdir path is not valid UTF-8"),
                ])
                .current_dir(&d)
                .status()
                .expect("failed to execute cargo build for fuse-overlayfs");
            if !status.success() {
                panic!("fuse-overlayfs build failed")
            }
            // The release binary is at outdir_abs/release/fuse-overlayfs
            let built_bin = outdir_abs.join("release").join(EXEC);
            if built_bin.exists() {
                std::fs::copy(&built_bin, &exec_path)
                    .expect("failed to copy fuse-overlayfs binary");
            } else {
                panic!(
                    "fuse-overlayfs binary not found at expected path: {}",
                    built_bin.display()
                );
            }
        };
        #[cfg(feature = "build-cache")]
        if vendored::need_rebuild(&d, &outdir, vec![]) {
            build();
        }
        #[cfg(not(feature = "build-cache"))]
        build();
        println!(
            "cargo:rustc-env=FUSE-OVERLAYFS-BIN={}",
            exec_path.to_string_lossy()
        );
    }

    #[cfg(all(feature = "unionfs-fuse-vendored", target_os = "linux"))]
    {
        println!("cargo:rerun-if-changed=patches/unionfs-fuse/erofs-fix.patch");
        let source_dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/unionfs-fuse/");
        if !source_dir.exists() && vendored::download_submodule().is_err() {
            panic!("unionfs-fuse submodule is not present unionfs-fuse-vendored cannot be used")
        }

        const EXEC: &str = "unionfs";
        let outdir = std::path::Path::new("./target/vendored").join(EXEC);
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir)
                .expect("failed to create output directory for unionfs-fuse")
        }
        let exec_path = outdir.join("build/src").join(EXEC);

        // Copy vendor source to target/ so patches don't taint the git-tracked vendor directory
        let build_dir = std::path::Path::new("./target/vendored").join("unionfs-fuse-source");

        let build = || {
            // Copy the vendor source to target/
            let _ = std::fs::remove_dir_all(&build_dir);
            vendored::copy_dir(&source_dir, &build_dir);

            // Apply the EROFS fix patch before building (idempotent)
            let patch_file =
                env!("CARGO_MANIFEST_DIR").to_string() + "/patches/unionfs-fuse/erofs-fix.patch";
            // First check if patch is already applied using dry-run
            let dry_run = std::process::Command::new("patch")
                .args(["-p1", "--dry-run", "-i", &patch_file])
                .current_dir(&build_dir)
                .output()
                .expect("failed to execute patch dry-run for unionfs-fuse");
            // If dry-run fails (patch already applied), reverse and re-apply to ensure clean state
            if !dry_run.status.success() {
                // Patch already applied - try reversing first to get clean state
                let reverse_status = std::process::Command::new("patch")
                    .args(["-p1", "--reverse", "-i", &patch_file])
                    .current_dir(&build_dir)
                    .status()
                    .expect("failed to reverse unionfs-fuse patch");
                if reverse_status.success() {
                    // Now apply fresh
                    let status = std::process::Command::new("patch")
                        .args(["-p1", "-i", &patch_file])
                        .current_dir(&build_dir)
                        .status()
                        .expect("failed to apply unionfs-fuse patch");
                    if !status.success() {
                        panic!("failed to apply unionfs-fuse patch");
                    }
                }
                // If reverse also failed, patch is likely in a clean state already
            } else {
                // Patch not yet applied, apply it
                let status = std::process::Command::new("patch")
                    .args(["-p1", "-i", &patch_file])
                    .current_dir(&build_dir)
                    .status()
                    .expect("failed to apply unionfs-fuse patch");
                if !status.success() {
                    panic!("failed to apply unionfs-fuse patch");
                }
            }
            cmake::Config::new(&build_dir)
                .out_dir(
                    std::fs::canonicalize(&outdir)
                        .expect("cannot canonicalize outdir for unionfs-fuse"),
                )
                .very_verbose(false)
                .build();
        };
        #[cfg(feature = "build-cache")]
        if vendored::need_rebuild(&source_dir, &outdir, vec![]) {
            build();
        }
        #[cfg(not(feature = "build-cache"))]
        build();
        println!(
            "cargo:rustc-env=UNIONFS-FUSE-BIN={}",
            exec_path.to_string_lossy()
        );
    }
}

#[cfg(any(feature = "fuse-overlayfs-vendored", feature = "unionfs-fuse-vendored"))]
mod vendored {
    /// Recursively copy a directory from `src` to `dst`.
    #[cfg(feature = "unionfs-fuse-vendored")]
    pub fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).expect("failed to create destination directory for copy_dir");
        for entry in std::fs::read_dir(src).expect("failed to read source directory") {
            let entry = entry.expect("failed to read directory entry");
            let file_type = entry.file_type().expect("failed to get file type");
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if file_type.is_dir() {
                copy_dir(&src_path, &dst_path);
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .expect("failed to copy file during directory copy");
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn download_submodule() -> std::io::Result<()> {
        if std::process::Command::new("git")
            .args(["submodule", "update", "--init", "--recursive"])
            .spawn()?
            .wait()?
            .success()
        {
            return Ok(());
        }
        Err(std::io::Error::other("Cannot get submodule"))
    }

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
            for entry in std::fs::read_dir(src).expect("failed to read directory") {
                let entry = entry.expect("failed to read directory entry");
                let path = entry.path();
                if entry.file_type().expect("failed to get file type").is_dir() {
                    check_dir(hashed, &path, exception);
                    continue;
                }
                let path_str = path.to_string_lossy().to_string();
                if !exception.contains(&path_str) {
                    // Stream the file through the hasher instead of loading it entirely into memory
                    let file = std::fs::File::open(src.join(&path_str))
                        .expect("failed to open file for hashing");
                    let mut hasher = md5::Context::new();
                    std::io::copy(&mut std::io::BufReader::new(file), &mut hasher)
                        .expect("failed to hash file");
                    #[allow(deprecated)]
                    let hash = hasher.compute().0;
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
            std::fs::write(
                cache_file,
                serde_json::to_string(&new_cache).expect("failed to serialize cache"),
            )
            .expect("Unable to write cache file");
            true
        } else {
            println!("No changes detected in vendored dependency.");
            false
        }
    }
}
