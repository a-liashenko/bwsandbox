use std::process::ExitCode;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod args;
mod config;
mod error;
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

    let args = match args::Args::from_iter(std::env::args_os()) {
        Ok(v) => v,
        Err(e) => {
            print_error(&e);
            return print_help();
        }
    };

    // Should work for appimage v2
    // https://github.com/AppImage/AppImageKit/issues/841
    if args.is_app_image {
        unsafe { std::env::set_var("APPIMAGE_EXTRACT_AND_RUN", "1") };
    }

    if let Err(e) = run(args) {
        print_error(&e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(args: args::Args) -> Result<(), error::AppError> {
    let mut app = app::App::from_str(&args.config)?;
    app.apply_services()?;

    let status = app.run(args.app, args.app_args.into_iter())?;
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
