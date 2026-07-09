//! The `chronos-fm` binary: parses CLI arguments, runs headless `config` subcommands
//! when requested, and otherwise launches the gpui desktop application.

mod app;
mod cli;

use clap::Parser;

fn main() {
    // Read scale early (WINIT_SCALE_FACTOR is picked up by winit automatically)
    let _chronos_fm_scale = parse_chronos_fm_scale();

    // TODO: When GPUI adds App::set_global_scale_factor(), use it here:
    // if let Some(scale) = _chronos_fm_scale {
    //     gpui::App::set_global_scale_factor(scale);
    // }

    let cli = cli::Cli::parse();

    // `config` subcommands run headless and exit without opening a window.
    if let Some(cli::Command::Config { action }) = &cli.command {
        chronos_fm_core::telemetry::logging::init_logging();
        std::process::exit(cli::run_config_command(action, &cli));
    }

    app::ChronosFmApp::run(&cli);
}

/// Parses the `CHRONOS_FM_SCALE` environment variable for HiDPI scaling.
///
/// Returns `Some(scale)` if the variable is set to a valid f64 in the range (0.0, 5.0],
/// otherwise returns `None`. This is a workaround until GPUI adds dynamic HiDPI scaling support.
fn parse_chronos_fm_scale() -> Option<f64> {
    std::env::var("CHRONOS_FM_SCALE")
        .ok()
        .and_then(parse_chronos_fm_scale_str)
}

/// Parses a scale factor string, returning `Some(scale)` if valid in range (0.0, 5.0].
fn parse_chronos_fm_scale_str(s: String) -> Option<f64> {
    s.parse::<f64>().ok().filter(|&v| v > 0.0 && v <= 5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chronos_fm_scale_str_valid() {
        assert_eq!(parse_chronos_fm_scale_str("1.5".to_string()), Some(1.5));
        assert_eq!(parse_chronos_fm_scale_str("2.0".to_string()), Some(2.0));
        assert_eq!(parse_chronos_fm_scale_str("0.5".to_string()), Some(0.5));
        assert_eq!(parse_chronos_fm_scale_str("5.0".to_string()), Some(5.0));
    }

    #[test]
    fn test_parse_chronos_fm_scale_str_invalid() {
        assert_eq!(parse_chronos_fm_scale_str("not_a_number".to_string()), None);
        assert_eq!(parse_chronos_fm_scale_str("".to_string()), None);
        assert_eq!(parse_chronos_fm_scale_str("abc".to_string()), None);
    }

    #[test]
    fn test_parse_chronos_fm_scale_str_out_of_range() {
        assert_eq!(parse_chronos_fm_scale_str("10.0".to_string()), None);
        assert_eq!(parse_chronos_fm_scale_str("0.0".to_string()), None);
        assert_eq!(parse_chronos_fm_scale_str("-1.0".to_string()), None);
        assert_eq!(parse_chronos_fm_scale_str("-0.5".to_string()), None);
    }

    #[test]
    fn test_parse_chronos_fm_scale_not_set() {
        // This test just verifies the function signature compiles correctly.
        // The actual environment variable reading is tested via the pure function above.
        let _ = parse_chronos_fm_scale();
    }
}
