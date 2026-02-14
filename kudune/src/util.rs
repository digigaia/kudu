use std::process;

use color_eyre::eyre::{eyre, Report};
use color_eyre::{Section, SectionExt};
use tracing::debug;


#[track_caller]
pub fn eyre_from_output(msg: &str, output: &process::Output) -> Report {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eyre!(msg.to_string())
        .with_section(move || stdout.trim().to_string().header(format!("{:━^80}", " STDOUT ")))
        .with_section(move || stderr.trim().to_string().header(format!("{:━^80}", " STDERR ")))
}


#[macro_export]
macro_rules! print_streams {
    ($level:ident, $output:expr) => {
        let output = $output;
        let stdout = std::str::from_utf8(&output.stdout).expect("stdout is invalid utf-8!!");
        let stderr = std::str::from_utf8(&output.stderr).expect("stderr is invalid utf-8!!");

        if !stdout.is_empty() {
            $level!("================ STDOUT ================\n{}", stdout);
        }
        if !stderr.is_empty() {
            $level!("================ STDERR ================\n{}", stderr);
        }
        if stdout.is_empty() && stderr.is_empty() {
            $level!("=============== NO OUTPUT ==============");
        }
        $level!("========================================");
    };
}


pub enum QuoteType {
    Single,
    Double,
    Unspecified,
    None
}

pub fn need_quote(s: &str) -> QuoteType {
    let has_single = s.contains('\'');
    let has_double = s.contains('"');

    // one type of quote in the string, need the other one
    if has_single && !has_double { return QuoteType::Double; }
    if has_double && !has_single { return QuoteType::Single; }

    // no quotes in string, only need quotes if there are whitespaces
    if !has_single && !has_double {
        if s.contains(char::is_whitespace) {
            return QuoteType::Unspecified;
        }
        else {
            return QuoteType::None;
        }
    }

    // both types of quotes in string, unspecified -> need further refinement
    if has_single && has_double {
        debug!("String needs quote but we're not sure which type: {}", s);
        return QuoteType::Unspecified;
    }

    QuoteType::None
}

pub fn quote_if_needed(s: &str) -> String {
    match need_quote(s) {
        QuoteType::Single => format!("'{s}'"),
        QuoteType::Double |
        QuoteType::Unspecified => format!("\"{s}\""),
        QuoteType::None => s.to_string()
    }
}

pub fn join_quote(args: &[&str]) -> String {
    let args: Vec<_> = args.iter()
        .map(|s| quote_if_needed(s))
        .collect();

    args.join(" ")
}
