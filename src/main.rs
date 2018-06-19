#![feature(fs_read_write)]

#[macro_use]
extern crate clap;
extern crate rustyline;
extern crate funqy;

use std::env;
use std::fs;
use rustyline::error::ReadlineError;
use rustyline::Editor;

use funqy::{parser, eval, stdlib};

fn main() {
	let matches = clap_app!(funqy =>
		(author: "Ryan Vandersmith (https://github.com/rvanasa)")
		(about: "FunQy language command-line interface")
		(@subcommand run =>
			(about: "execute script using simulator")
			(@arg filename: +required "input filename")
			(@arg output: -o --output +takes_value "output filename")
		)
		(@subcommand repl =>
			(about: "begin REPL session")
			(@arg history: -h --history +takes_value "history file")
		)
	).get_matches();
	
	let mut ctx = stdlib::create_ctx(env::current_dir()
		.expect("Could not find working directory")
		.to_str().unwrap());
	
	if let Some(matches) = matches.subcommand_matches("run") {
		let result = ctx.import(matches.value_of("filename").unwrap());
		println!(">> {}", result);
		
		if let Some(output) = matches.value_of("output") {
			fs::write(output, format!("{}", result))
				.expect("Could not write output file");
		}
	}
	else if let Some(matches) = matches.subcommand_matches("repl") {
		let mut rl = Editor::<()>::new();
		let history = if matches.is_present("history") {
			matches.value_of("history")
		} else {None};
		if let Some(file) = history {
			if rl.load_history(file).is_err() {
				println!("No previous history found.");
			}
		}
		loop {
			match rl.readline(": ") {
				Ok(line) => {
					rl.add_history_entry(line.as_ref());
					match parser::parse(line) {
						Ok(exp) => {
							let result = eval::eval_exp_inline(&exp, &mut ctx);
							if result != eval::RunVal::Tuple(vec![]) {
								println!(">> {}", result);
							}
						},
						Err(err) => println!("Error: {:?}", err),
					}
				},
				Err(ReadlineError::Interrupted) => break,
				Err(ReadlineError::Eof) => break,
				Err(err) => {println!("Terminated: {:?}", err); break},
			}
		}
		if let Some(file) = history {
			rl.save_history(file).unwrap();
		}
	}
	else {
		panic!("Invalid subcommand");
	}
}