//! Print the `config.toml` JSON Schema to stdout.
//!
//! This emits the exact bytes committed to `docs/config.schema.json` (and the
//! same output as `chronos-fm config schema`), but builds only `chronos-fm-core`, so it
//! runs on Linux where the gpui-backed `chronos-fm` binary does not link. CI uses it
//! to keep the committed schema in sync:
//!
//! ```bash
//! cargo run -p chronos-fm-core --example schema > docs/config.schema.json
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", chronos_fm_core::config::json_schema_string()?);
    Ok(())
}
