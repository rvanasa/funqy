extern crate funqy;
use funqy::stdlib::*;

#[test]
fn test_parser() {
	let ctx = create_ctx("tests/scripts");
	// println!("{:?}", exp);
	println!("\n>> {}\n", ctx.import_eval("Test")/*.expect("Could not parse file")*/);
}
