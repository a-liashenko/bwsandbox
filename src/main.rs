use std::{ffi::OsString, fmt::Debug, process::ExitCode};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod bwrap;
mod config;
mod error;
mod fd;
mod services;
mod utils;

fn main() -> ExitCode {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_timer(tracing_subscriber::fmt::time::uptime())
                .with_target(true),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .init();

    let mut args = std::env::args_os().skip(1).peekable();
    let first_arg = args.peek().map(|v| v.to_string_lossy());
    let result = match first_arg.as_deref() {
        Some(utils::SELF_INTERNAL_ARG) => run_bwrap(args),
        _ => run(args),
    };

    if let Err(e) = result {
        print_error(&e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[tracing::instrument]
fn run<I: Iterator<Item = OsString> + std::fmt::Debug>(args: I) -> Result<(), error::AppError> {
    let _span = tracing::trace_span!("[orchestartor]").entered();
    let args = match app::Args::from_iter(args) {
        Ok(v) => v,
        Err(e) => {
            print_error(&e);
            print_help();
            return Ok(());
        }
    };

    // Should work for appimage v2
    // https://github.com/AppImage/AppImageKit/issues/841
    if args.is_app_image {
        unsafe { std::env::set_var("APPIMAGE_EXTRACT_AND_RUN", "1") };
    }

    let status = app::App::start(args)?;
    std::process::exit(status.code().unwrap_or(-1));
}

#[tracing::instrument(skip(args))]
fn run_bwrap<I: Iterator<Item = OsString> + Debug>(args: I) -> Result<(), error::AppError> {
    let _span = tracing::trace_span!("[bwrap]").entered();

    let args = bwrap::Args::from_iter(args).expect("Internal spawn args must be valid");
    let status = bwrap::BwrapRunner::new(args).run()?;
    std::process::exit(status.code().unwrap_or(-1));
}

fn print_error(e: &error::AppError) {
    tracing::info!("{e:#?}");
    tracing::error!("{}", e.to_string());
}

fn print_help() -> ExitCode {
    println!("-----------------");
    println!("Usage: {} --config-name app -- app --flag", utils::APP_NAME);
    println!("-----------------");
    ExitCode::SUCCESS
}
