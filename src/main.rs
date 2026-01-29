use std::process::{ExitCode, ExitStatus};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod bwrap;
mod config;
mod error;
mod fd;
mod services;
mod temp_dir;
mod utils;

#[cfg(test)]
mod tests;

fn main() -> ExitCode {
    // If NO_COLOR not set or invalid => enable colors
    let no_color = std::env::var("NO_COLOR").is_ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(!no_color)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_timer(tracing_subscriber::fmt::time::uptime())
                .with_target(true),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .init();

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

fn run(args: app::Args) -> Result<ExitStatus, error::AppError> {
    let _guard = temp_dir::TempDirGuard::new(utils::temp_dir())?;
    let status = app::App::start(args)?;
    Ok(status)
}

fn print_error(e: &error::AppError) {
    tracing::info!("{e:#?}");
    tracing::error!("{}", e.to_string());
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
