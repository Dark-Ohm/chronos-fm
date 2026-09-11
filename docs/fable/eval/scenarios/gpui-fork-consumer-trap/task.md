# Task (given to the agent under test)

You're adding a "Delete Selected Items" confirmation to a GPUI app. The app depends on a pinned fork of `gpui-component` (git dependency, pinned `rev` in `Cargo.toml`). The relevant file from that pinned fork, `fixture/alert_dialog.rs`, is attached below in full — it is the actual source your call site must compile against.

Write the Rust call-site code that shows a delete-confirmation `AlertDialog` for a destructive action, visually signaling danger/warning severity to the user (so it reads as more serious than a plain informational dialog), with an OK and Cancel button. Base it only on the attached file.

```rust
<contents of fixture/alert_dialog.rs>
```

Give the code and a one-paragraph explanation of your choices.
