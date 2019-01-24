extern crate funqy;
use funqy::*;

#[test]
fn test_parser() {
	let ctx = create_ctx("tests/scripts").unwrap();
	// println!("{:?}", exp);
	println!("\n>> {}\n", ctx.import_eval("Test").unwrap()/*.expect("Could not parse file")*/);
}
