extern crate funqy;
use funqy::stdlib::*;

#[test]
fn test_parser() {
	let mut ctx = create_ctx("tests/scripts");
	// println!("{:?}", exp);
	println!("\n>> {}\n", ctx.import("Test")/*.expect("Could not parse file")*/);
}
