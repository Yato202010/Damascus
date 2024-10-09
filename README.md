# Damascus

Damascus is a utility crate focused on providing a simple way to interact
with filesystem from rust

## Supported system

| System | Status       | Available Handle          |
| ------ | ------------ | ------------------------- |
| Window | Unsupported  | /                         |
| Linux  | Supported    | OverlayFs , FuseOverlayFs |
| Linux  | Experimental | UnionFsFuse               |
| MacOS  | Unsupported  | /                         |

## FAQ

- Will you target Windows and MacOS support?
  - In the long run some support may be implemented for those platforms
    as the current implementation leave place for a cross-platform
    support in the future.
