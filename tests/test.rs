#[macro_use]
extern crate funqy;
use funqy::ast::*;
use funqy::engine::*;
use funqy::eval::*;

use std::rc::Rc;

extern crate num;
use num::complex::Complex;

fn round(f: Cf32, d: i32) -> Cf32 {
	let m = real!(10_f32.powi(d));
	let f = f * m;
	Complex::new(f.re.round(), f.im.round()) / m
}

#[test]
fn test_eval() {
	
	let exp = Exp::Scope(vec![
		Decl::Data("Bool", vec!["F", "T"]),
		Decl::Let(Pat::Var("abc"), Exp::Var("T")),
	], Rc::new(Exp::State(Rc::new(Exp::Var("abc")))));
	
	let ctx = Context::new();
	let result = eval_exp(&exp, &ctx);
	
	println!("\n >> {}\n", result);
}

// #[test]
fn test_engine() {
	// fn zero() -> State {vec![real!(1)]}
	// fn one() -> State {vec![real!(0), real!(1)]}
	
	// fn not(s: S2) -> S2 {
	// 	s.extract(S2::one(), S2::zero())
	// }
	
	// fn had(s: S2) -> S2 {
	// 	s.extract(
	// 		S2::zero().sup(S2::one()),
	// 		S2::zero().sup(S2::one().phase_flip()))
	// }
	
	// fn cnot(a: S2, b: S2) -> S2 {
	// 	a.extract(
	// 		b.clone(),
	// 		not(b),
	// 	)
	// }
	
	// fn test4(s: State<S2>) -> State<S2> {
	// 	s.extract(
	// 		(S2::zero(), S2::zero().sup(S2::one())),
	// 		(S2::zero(), S2::zero()),
	// 	)
	// }
	
	// let s = had(had(S2::zero()));
	
	// let s = Zero.sup(One).extract(
	// 	One,
	// 	Zero,
	// );
	
	// let (x, y) = s;
	// println!("{} {}", round(x, 4), round(y, 4));
	
	// let a = get_state(3);
	// let b = get_state(2).phase_flip();
	let a = get_state(0);
	let b = get_state(1).phase_flip();
	let c = get_state(2);
	
	// let s = a.sup(b);
	let s = a.sup(get_state(1).phase_flip()).sup(get_state(2)).extract(vec![b, c]);
	
	let mut i = 0;
	let mag = s.iter().fold(real!(0), move |a, b| a + (b * b));
	println!("State: [{}] ({})", s.len(), mag);
	for x in s {
		let pow = if num::traits::Zero::is_zero(&x) {real!(0)} else {x * x};
		println!("{}  {}%\t{} ", i, (pow * real!(100) / mag).re.round() as usize, round(x, 4));
		i += 1;
	}
}