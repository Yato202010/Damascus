# Damascus

[![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/Yato202010/Damascus)](https://github.com/Yato202010/Damascus/issues)
[![GitHub License](https://img.shields.io/github/license/Yato202010/Damascus)](https://github.com/Yato202010/Damascus/blob/main/LICENSE)
[![docs.rs](https://img.shields.io/docsrs/damascus)](https://docs.rs/damascus/latest/damascus/)
[![Crates.io Version](https://img.shields.io/crates/v/damascus)](https://crates.io/crates/damascus)

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

fn main() {
    let lower1 = Path::new("lowest_layer");
    let lower2 = Path::new("lower_layer");
    let upper = Path::new("upper_layer");
    let work = Path::new("working");
    let target = Path::new("mount_target");
    let drop = true;

    // handle can be created using complex or simple interface based on need
    // NOTE : drop control if once dropped the filesystem should be unmounted
    let mut o = FuseOverlayFs::new(
        [&lower1, &lower2].iter(),
        Some(upper),
        Some(work),
        target,
        drop,
    )
    .unwrap();
    // or
    o = FuseOverlayFs::writable([&lower1, &lower2].iter(), upper, work, &target).unwrap();
    // or
    o = FuseOverlayFs::readonly([&lower1, &lower2].iter(), target).unwrap();

    o.add_option(FuseOverlayFsOption::AllowRoot).unwrap();
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
