mod error;
mod lexer;
mod parser;
mod pretty;

use std::env;
use std::fs;
use std::process::ExitCode;

use lexer::Lexer;
use parser::Parser;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("nomina");

    if args.len() != 3 {
        eprintln!("usage: {} <check|fmt> <file.nomina>", program);
        return ExitCode::FAILURE;
    }
    let command = args[1].as_str();
    let path = &args[2];

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read `{}`: {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    let tokens = match Lexer::new(&source).tokenize() {
        Ok(tokens) => tokens,
        Err(diag) => {
            eprint!("{}", diag.render(path, &source));
            return ExitCode::FAILURE;
        }
    };

    let grammar = match Parser::new(tokens).parse_grammar() {
        Ok(grammar) => grammar,
        Err(diag) => {
            eprint!("{}", diag.render(path, &source));
            return ExitCode::FAILURE;
        }
    };

    let diagnostics = parser::validate(&grammar);
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            eprint!("{}", diag.render(path, &source));
        }
        eprintln!("{} error(s) found", diagnostics.len());
        return ExitCode::FAILURE;
    }

    match command {
        "check" => {
            println!("{}: grammar is valid ({} rules)", path, grammar.rules.len());
            ExitCode::SUCCESS
        }
        "fmt" => {
            print!("{}", pretty::pretty_print(&grammar));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command `{}` (expected `check` or `fmt`)", other);
            ExitCode::FAILURE
        }
    }
}
