# Damascus
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/Yato202010/Damascus)
![GitHub License](https://img.shields.io/github/license/Yato202010/Damascus)
![docs.rs](https://img.shields.io/docsrs/damascus)
![Crates.io Version](https://img.shields.io/crates/v/damascus)

Damascus is a utility crate focused on providing a simple way to interact
with filesystem from rust

## Supported system

| System | Status       | Available Handle          |
| ------ | ------------ | ------------------------- |
| Window | Unsupported  | /                         |
| Linux  | Supported    | OverlayFs , FuseOverlayFs |
| Linux  | Experimental | UnionFsFuse               |
| MacOS  | Unsupported  | /                         |

## How to use ?

```rust
use damascus::{Filesystem, FuseOverlayFs, FuseOverlayFsOption, LinuxFilesystem, MountOption};

// handle can be created using complex or simple interface based on need
// NOTE : drop control if once dropped the filesystem should be unmounted
let mut o = FuseOverlayFs::new([&lower1, &lower2].iter(), Some(upper), Some(work), target, drop).unwrap();
// or
let mut o = FuseOverlayFs::writable([&lower1, &lower2].iter(), upper, work, &target).unwrap();
// or
let mut o = FuseOverlayFs::readonly([&lower1, &lower2].iter(), target).unwrap();

o.set_option(FuseOverlayFsOption::AllowRoot).unwrap();
o.set_unmount_on_drop(false); // true by default

// once configured you can mount it
o.mount().unwrap();

// and then unmount it
o.unmount().unwrap();
```

## FAQ

- Will you target Windows and MacOS support?
  - In the long run some support may be implemented for those platforms
    as the current implementation leave place for a cross-platform
    support in the future.
