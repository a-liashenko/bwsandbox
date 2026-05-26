use std::process::{ExitCode, ExitStatus};

mod app;
mod bwrap;
mod config;
mod error;
mod services;
mod system;
mod temp_dir;
mod utils;

mod print_command;
#[cfg(test)]
mod tests;

fn main() -> ExitCode {
    setup_log();
    let args = match app::Args::from_iter(std::env::args_os()) {
        Ok(v) => v,
        Err(e) => {
            print_error(&e);
            return print_help();
        }
    };

    if let Err(e) = run(args) {
        print_error(&e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn setup_log() {
    use env_logger::WriteStyle;

    // If NO_COLOR not set or invalid => enable colors
    let style = std::env::var("NO_COLOR").map_or(WriteStyle::Always, |_| WriteStyle::Never);

    env_logger::builder()
        .write_style(style)
        .format_source_path(true)
        .format_target(false)
        .init();
}

fn run(args: app::Args) -> Result<ExitStatus, error::AppError> {
    let _guard = temp_dir::TempDirGuard::new(utils::temp_dir())?;
    let status = app::App::start(args)?;
    Ok(status)
}

fn print_error(e: &error::AppError) {
    log::info!("{e:#?}");
    log::error!("{e}");
}

fn print_help() -> ExitCode {
    println!("-----------------");
    println!("Usage: {} [--flags] -- app --arg1 arg2", utils::APP_NAME);
    println!("\t-f, --config-file  <path to profile.toml>");
    println!("\t-n, --config-name  <profile name in $XDG_CONFIG_PATH/bwsandbox>");
    println!("\t-a, --config-auto");
    println!("\t\tWill use <app> as profile name in $XDG_CONFIG_PATH/bwsandbox");
    println!("-----------------");
    ExitCode::SUCCESS
}
