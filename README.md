# Damascus

[![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/Yato202010/Damascus)](https://github.com/Yato202010/Damascus/issues)
[![GitHub License](https://img.shields.io/github/license/Yato202010/Damascus)](https://github.com/Yato202010/Damascus/blob/main/LICENSE)
[![docs.rs](https://img.shields.io/docsrs/damascus)](https://docs.rs/damascus/latest/damascus/)
[![Crates.io Version](https://img.shields.io/crates/v/damascus)](https://crates.io/crates/damascus)
[![Matrix](https://img.shields.io/badge/Matrix-Join%20Chat-000000?logo=matrix&logoColor=white)](https://matrix.to/#/#flamberge-mo:matrix.org)

Damascus is a utility crate focused on providing a simple way to interact
with filesystem from rust

## Supported system

| System | Status       | Available Handle          |
| ------ | ------------ | ------------------------- |
| Window | Unsupported  | /                         |
| Linux  | Supported    | OverlayFs , FuseOverlayFs |
| Linux  | Experimental | UnionFsFuse               |
| MacOS  | Unsupported  | /                         |

## How to use?

```rust
use std::path::Path;
use damascus::{Filesystem, FuseOverlayFs, FuseOverlayFsOption, StateRecovery};
use temp_testdir::TempDir;
use std::fs::create_dir_all;

fn main() {
    let tmp = TempDir::default().to_path_buf();
    let lower1 = tmp.join("lowest_layer");
    let lower2 = tmp.join("lower_layer");
    let upper = tmp.join("upper_layer");
    let work = tmp.join("working");
    let target = tmp.join("mount_target");
    let drop = true;
    create_dir_all(&lower1).unwrap();
    create_dir_all(&lower2).unwrap();
    create_dir_all(&upper).unwrap();
    create_dir_all(&work).unwrap();
    create_dir_all(&target).unwrap();

    // handle can be created using complex or simple interface based on need
    // NOTE : drop control if once dropped the filesystem should be unmounted ie scoped mount
    let mut o = FuseOverlayFs::new(
        [&lower1, &lower2],
        Some(&upper),
        Some(&work),
        &target,
        drop,
    )
    .unwrap();
    // or
    o = FuseOverlayFs::writable([&lower1, &lower2], upper, work, &target).unwrap();
    // or
    o = FuseOverlayFs::readonly([&lower1, &lower2], &target).unwrap();

    o.add_option(FuseOverlayFsOption::CloneFd).unwrap();
    o.set_scoped(false); // true by default

    // once configured you can mount it
    o.mount().unwrap();

    // if handle is lost it can be recovered from system information
    let recovered = FuseOverlayFs::recover(target).unwrap();

    // and then unmount it
    o.unmount().unwrap();
}
```

## FAQ

- Will you target Windows and MacOS support?
  - In the long run some support may be implemented for those platforms
    as the current implementation leave place for a cross-platform
    support in the future.
