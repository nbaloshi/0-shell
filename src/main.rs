mod commands;

use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().ok();

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input);

        match bytes_read {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!();
                break;
            }
            Ok(_) => {
                let line = input.trim();
                if line.is_empty() {
                    continue;
                }

                let args = match parse_line(line) {
                    Ok(args) => args,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };

                if args.is_empty() {
                    continue;
                }

                let command = &args[0];
                let rest = &args[1..];

                match command.as_str() {
                    "exit" => break,
                    "echo" => commands::echo(rest),
                    "cd" => commands::cd(rest),
                    "pwd" => commands::pwd(rest),
                    "ls" => commands::ls(rest),
                    "cat" => commands::cat(rest),
                    "cp" => commands::cp(rest),
                    "rm" => commands::rm(rest),
                    "mv" => commands::mv(rest),
                    "mkdir" => commands::mkdir(rest),
                    other => println!("Command '{}' not found", other),
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

/// Splits a line of input into arguments, respecting single and double
/// quotes (so `echo "a b"` produces one argument `a b`).
fn parse_line(line: &str) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut has_token = false;

    for c in line.chars() {
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            } else {
                current.push(c);
            }
        } else if in_double_quote {
            if c == '"' {
                in_double_quote = false;
            } else {
                current.push(c);
            }
        } else if c == '\'' {
            in_single_quote = true;
            has_token = true;
        } else if c == '"' {
            in_double_quote = true;
            has_token = true;
        } else if c.is_whitespace() {
            if has_token {
                args.push(std::mem::take(&mut current));
                has_token = false;
            }
        } else {
            current.push(c);
            has_token = true;
        }
    }

    if in_single_quote || in_double_quote {
        return Err("Unmatched quote".to_string());
    }

    if has_token {
        args.push(current);
    }

    Ok(args)
}
