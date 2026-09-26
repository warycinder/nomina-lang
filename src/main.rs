mod error;
mod generate;
mod lexer;
mod parser;
mod pretty;

use std::env;
use std::fs;
use std::process::ExitCode;

use generate::Rng;
use lexer::Lexer;
use parser::Parser;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("nomina");

    if args.len() < 3 {
        eprintln!("usage: {} <check|fmt|gen> <file.nomina> [count]", program);
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
        "gen" => {
            let count: usize = match args.get(3) {
                Some(s) => match s.parse() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        eprintln!("error: count must be a positive integer, got `{}`", s);
                        return ExitCode::FAILURE;
                    }
                },
                None => 1,
            };
            let mut rng = Rng::new();
            for _ in 0..count {
                match generate::generate(&grammar, &mut rng) {
                    Ok(name) => println!("{}", name),
                    Err(_) => {
                        eprintln!(
                            "error: `name` recurses more than {} rule references deep without \
                             reaching a literal; check for a rule that (indirectly) refers to \
                             itself with no base case",
                            generate::MAX_DEPTH
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command `{}` (expected `check`, `fmt`, or `gen`)", other);
            ExitCode::FAILURE
        }
    }
}
