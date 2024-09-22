# Damascus

Damascus is a utility crate focused on providing a simple way to interact
with filesystem for the Flamberg mod manager stack

## Supported system

| System | Status      | Available Handle          |
|--------|-------------|---------------------------|
| Window | Unsupported | /                         |
| Linux  | Supported   | OverlayFs , FuseOverlayFs |
| macOS  | Unsupported | /                         |

## FAQ

- Can I use it in my project ?
    - Yes you can but keep in mind that bug which don't affect the usage
      made by Flamberg will probably not be fixed with high priority!
      Pull request are welcome for fixing those.
- Will you target Windows and macOS support ?
    - In the long run some support may be implemented for those platforms
      as the current implementation leave place for a cross-platform
      support in the future.

## Flamberg mod manager stack

- [Damascus : Overlay filesystem utility crate](https://github.com/Yato202010/Damascus)
