use std::{ffi::OsString, fmt::Debug, process::ExitCode};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod args;
mod config;
mod error;
mod fd;
mod service;
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
        Some(utils::SELF_INTERNAL_ARG) => run_internal(args),
        _ => run(args),
    };

    if let Err(e) = result {
        print_error(&e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run<I: Iterator<Item = OsString>>(args: I) -> Result<(), error::AppError> {
    tracing::trace_span!("[parent]");
    let args = match args::Args::from_iter(args) {
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

    let mut app = app::App::try_parse(&args.config)?;
    app.apply_services()?;

    let status = app.run(args.app, args.app_args.into_iter())?;
    std::process::exit(status.code().unwrap_or(-1));
}

#[tracing::instrument(skip(args))]
fn run_internal<I: Iterator<Item = OsString> + Debug>(args: I) -> Result<(), error::AppError> {
    tracing::trace_span!("[child]");
    let args = args::InternalArgs::from_iter(args).expect("Internal spawn args must be valid");
    let app = app::InternalApp::new(args);

    let status = app.run()?;
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
