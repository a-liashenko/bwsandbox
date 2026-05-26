use lexopt::Parser;
use std::process::Command;

fn collect_args(parser: &mut Parser) -> String {
    let iter = parser.values();
    let Ok(iter) = iter else {
        // Usually it means that arg doesnt have any arguments
        return String::new();
    };

    iter.map(|v| v.display().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn print_command(cmd: &Command) {
    use lexopt::Arg::{Long, Short, Value};

    if !log::log_enabled!(log::Level::Trace) {
        return;
    }

    log::trace!("---- {} command ----", cmd.get_program().display());

    let mut parser = lexopt::Parser::from_args(cmd.get_args());

    while let Ok(Some(arg)) = parser.next() {
        match arg {
            Short(short) => {
                let args = collect_args(&mut parser);
                log::trace!("-{short} {}", args);
            }
            Long(long) => {
                let long = long.to_owned();
                let args = collect_args(&mut parser);
                log::trace!("--{long} {}", args);
            }
            Value(v) => {
                log::trace!("{}", v.display());
            }
        }
    }
}
