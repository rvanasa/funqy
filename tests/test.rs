extern crate funqy;
use funqy::ast::*;
use funqy::engine::*;
use funqy::eval::*;
use funqy::parser::*;

#[test]
fn test_parser() {

	fn lib_sup(exp: &Exp, ctx: &Context) -> RunVal {
		RunVal::State(match exp {
			&Exp::Tuple(ref args) => create_sup(args.iter().map(|arg| build_state(eval_exp(arg, ctx))).collect()),
			_ => build_state(eval_exp(exp, ctx)),
		})
	}
	
	fn lib_phf(exp: &Exp, ctx: &Context) -> RunVal {
		RunVal::State(build_state(eval_exp(exp, ctx)).phase_flip())
	}
	
	fn lib_measure(exp: &Exp, ctx: &Context) -> RunVal {
		RunVal::Index(build_state(eval_exp(exp, ctx)).measure())
	}
	
	fn lib_gate(exp: &Exp, ctx: &Context) -> RunVal {
		RunVal::Tuple(eval_gate(eval_exp(exp, ctx), ctx).into_iter().map(RunVal::State).collect())
	}
	
	fn lib_inv(exp: &Exp, ctx: &Context) -> RunVal {
		RunVal::Gate(eval_gate(eval_exp(exp, ctx), ctx).inverse())
	}
	
	let exp = parse_file("tests/scripts/Test.fqy").expect("Could not parse file");
	let mut ctx = Context::new();
	ctx.add_macro("sup", &lib_sup);
	ctx.add_macro("phf", &lib_phf);
	ctx.add_macro("measure", &lib_measure);
	ctx.add_macro("gate", &lib_gate);
	ctx.add_macro("inv", &lib_inv);
	
	println!("{:?}", exp);
	println!("\n>> {}\n", eval_exp(&exp, &ctx));
}

// // #[test]
// fn test_eval() {

// 	// let a = Exp::Tuple(vec![
// 	// 	Exp::Var("x"),
// 	// 	Exp::Var("y"),
// 	// ]);
	
// 	// let b = Exp::Tuple(vec![
// 	// 	Exp::Var("F"),
// 	// 	Exp::Var("T"),
// 	// ]);
	
// 	// let ret = Exp::Extract(Rc::new(Exp::Var("state")), vec![
// 	// 	Exp::Var("F"),
// 	// 	Exp::Tuple(vec![Exp::Var("F"), Exp::Var("T")]),
// 	// 	Exp::Tuple(vec![Exp::Var("T"), Exp::Var("F")]),
// 	// 	Exp::Var("T"),
// 	// ]);
	
// 	// let exp = Exp::Scope(vec![
// 	// 	Decl::Data("Bool", vec!["F", "T"]),
// 	// 	Decl::Let(Pat::Var("x"), Exp::Var("T")),
// 	// 	Decl::Let(Pat::Var("y"), Exp::Var("F")),
// 	// 	Decl::Let(Pat::Var("state"), Exp::Sup(Rc::new(a), Rc::new(b))),
// 	// ], Rc::new(Exp::Tuple(vec![
// 	// 	ret.clone(),
// 	// 	Exp::Measure(Rc::new(ret)),
// 	// ])));
	
// 	let exp = Exp::Tuple(vec![]);
	
// 	let ctx = Context::new();
// 	let result = eval_exp(&exp, &ctx);
	
// 	println!("\n >> {}\n", result);
// }

// // #[test]
// fn test_engine() {
// 	// fn zero() -> State {vec![real!(1)]}
// 	// fn one() -> State {vec![real!(0), real!(1)]}
	
// 	// fn not(s: S2) -> S2 {
// 	// 	s.extract(S2::one(), S2::zero())
// 	// }
	
// 	// fn had(s: S2) -> S2 {
// 	// 	s.extract(
// 	// 		S2::zero().sup(S2::one()),
// 	// 		S2::zero().sup(S2::one().phase_flip()))
// 	// }
	
// 	// fn cnot(a: S2, b: S2) -> S2 {
// 	// 	a.extract(
// 	// 		b.clone(),
// 	// 		not(b),
// 	// 	)
// 	// }
	
// 	// fn test4(s: State<S2>) -> State<S2> {
// 	// 	s.extract(
// 	// 		(S2::zero(), S2::zero().sup(S2::one())),
// 	// 		(S2::zero(), S2::zero()),
// 	// 	)
// 	// }
	
// 	// let s = had(had(S2::zero()));
	
// 	// let s = Zero.sup(One).extract(
// 	// 	One,
// 	// 	Zero,
// 	// );
	
// 	// let (x, y) = s;
// 	// println!("{} {}", round(x, 4), round(y, 4));
	
// 	// let a = get_state(3);
// 	// let b = get_state(2).phase_flip();
// 	let a = get_state(0);
// 	let b = get_state(1).phase_flip();
// 	let c = get_state(2);
	
// 	// let s = a.sup(b);
// 	// let s = a.sup(get_state(1).phase_flip()).sup(get_state(2)).extract(vec![b, c]);
	
// 	// let mut i = 0;
// 	// let mag = s.iter().fold(real!(0), |a, b| a + (b * b));
// 	// println!("State: [{}] ({})", s.len(), mag);
// 	// for x in s {
// 	// 	let pow = if num::traits::Zero::is_zero(&x) {real!(0)} else {x * x};
// 	// 	println!("{}  {}%\t{} ", i, (pow * real!(100) / mag).re.round() as usize, round(x, 4));
// 	// 	i += 1;
// 	// }
// }