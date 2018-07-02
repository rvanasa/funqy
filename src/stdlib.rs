use error::*;
use ast::Exp;
use engine::*;
use eval::*;

pub fn create_ctx(path: &str) -> Context {
	let mut ctx = Context::new(path.to_string());
	ctx.add_macro("import", &lib_import);
	ctx.add_macro("sup", &lib_sup);
	ctx.add_macro("phf", &lib_phf);
	ctx.add_macro("gate", &lib_gate);
	ctx.add_macro("inv", &lib_inv);
	ctx.add_macro("len", &lib_len);
	ctx.add_macro("slice", &lib_slice);
	ctx.add_macro("weighted", &lib_weighted);
	ctx.add_macro("fourier", &lib_fourier);
	ctx.add_macro("repeat", &lib_repeat);
	ctx.add_macro("measure", &lib_measure);
	ctx
}

fn lib_import(exp: &Exp, ctx: &Context) -> RunVal {
	match eval_exp(exp, ctx) {
		RunVal::String(ref s) => ctx.import_eval(s.as_str()),
		_ => panic!("Invalid import path"),
	}
}

fn lib_sup(exp: &Exp, ctx: &Context) -> RunVal {
	RunVal::State(match exp {
		&Exp::Tuple(ref args) => create_sup(args.iter().map(|arg| build_state(eval_exp(arg, ctx))).collect()),
		_ => build_state(eval_exp(exp, ctx)),
	})
}

fn lib_phf(exp: &Exp, ctx: &Context) -> RunVal {
	let val = eval_exp(exp, ctx);
	build_gate(&val, ctx)
		.map(|g| RunVal::Gate(g.negate()))
		.unwrap_or_else(|| RunVal::State(build_state(val).phase_flip()))
}

fn lib_gate(exp: &Exp, ctx: &Context) -> RunVal {
	let val = eval_exp(exp, ctx);
	RunVal::Tuple(build_gate(&val, ctx).unwrap_or_else(|| panic!("Not a gate: {}", val)).into_iter().map(RunVal::State).collect())
}

fn lib_inv(exp: &Exp, ctx: &Context) -> RunVal {
	let val = eval_exp(exp, ctx);
	RunVal::Gate(build_gate(&val, ctx).unwrap_or_else(|| panic!("Not a gate: {}", val)).inverse())
}

fn lib_len(exp: &Exp, ctx: &Context) -> RunVal {
	let val = eval_exp(exp, ctx);
	RunVal::Index(build_gate(&val, ctx)
		.map(|g| g.len())
		.unwrap_or_else(|| build_state(val).len()))
}

fn lib_slice(exp: &Exp, ctx: &Context) -> RunVal {
	fn to_slice_params(val: RunVal) -> Ret<(usize, usize)> {
		match val {
			RunVal::Index(n) => Ok((0, n)),
			RunVal::Tuple(args) => {
				if let [RunVal::Index(a), RunVal::Index(b)] = args[..] {
					if a <= b {Ok((a, b))}
					else {Err(Error(format!("Invalid slice arguments: {} > {}", a, b)))}
				}
				else {Err(Error(format!("Invalid slice arguments")))}
			},
			_ => Err(Error(format!("Invalid slice: {}", val))),
		}
	}
	match exp {
		&Exp::Tuple(ref args) if args.len() == 2 => {
			let state = build_state(eval_exp(&args[0], ctx));
			let (a, b) = to_slice_params(eval_exp(&args[1], ctx)).unwrap();
			RunVal::State(state.into_iter().chain(::std::iter::repeat(::num::Zero::zero())).skip(a).take(b - a).collect())
		},
		_ => panic!("Invalid `slice` arguments"),
	}
}

fn lib_weighted(exp: &Exp, ctx: &Context) -> RunVal {
	match exp {
		&Exp::Tuple(ref args) => {
			let weights: State = args.iter().map(|arg| {
				let val = eval_exp(arg, ctx);
				if let RunVal::Index(n) = val {Cf32::new(n as f32, 0_f32)}
				else {panic!("Invalid weight: {}", val)}
			}).collect();
			let div = weights.iter().fold(Cf32::new(0_f32, 0_f32), |a, b| a + b).sqrt();
			RunVal::State(weights.into_iter().map(|w| w.sqrt() / div).collect())
		},
		_ => panic!("Invalid `weighted` arguments"),
	}
}

fn lib_fourier(exp: &Exp, ctx: &Context) -> RunVal {
	// let state = build_state(eval_exp(exp, ctx));
	// let len = state.len();
	// let w = (2_f32 * ::std::f32::consts::PI * Cf32::i() / len as f32).exp();
	// let div = (len as f32).sqrt();
	// RunVal::State((0..len)
	// 	.map(|i| state.iter().enumerate()
	// 		.map(|(j, s)| s * w.powc(Cf32::new((i * j) as f32, 0_f32)))
	// 		.fold(Cf32::new(0_f32, 0_f32), |a, b| a + b) / div)
	// 	.collect())
	match eval_exp(exp, ctx) {
		RunVal::Index(n) if n > 0 => {
			let w = (2_f32 * ::std::f32::consts::PI * Cf32::i() / n as f32).exp();
			RunVal::Gate((0..n)
				.map(|i| (0..n)
					.map(|j| w.powc(Cf32::new((i * j) as f32, 0_f32)))
					.collect())
				.collect())
		},
		val => panic!("Invalid size argument: {}", val),
	}
}

fn lib_repeat(exp: &Exp, ctx: &Context) -> RunVal {
	fn do_repeat(state: State, n: usize) -> State {
		let div = (n as f32).sqrt();
		(0..n).flat_map(|_| state.iter().map(|s| s / div)).collect()
	}
	match exp {
		&Exp::Tuple(ref args) if args.len() == 2 => {
			let val = eval_exp(&args[0], ctx);
			match eval_exp(&args[1], ctx) {
				RunVal::Index(n) => {
					if let Some(gate) = build_gate(&val, ctx) {
						let wide = gate.into_iter().map(|v| do_repeat(v, n)).collect();
						RunVal::Gate(::std::iter::repeat(wide).take(n).flat_map(|g: Gate| g).collect())
					}
					else {
						RunVal::State(do_repeat(build_state(val), n))
					}
				},
				_ => panic!("Invalid `repeat` count"),
			}
		},
		_ => panic!("Invalid `repeat` arguments"),
	}
}

fn lib_measure(exp: &Exp, ctx: &Context) -> RunVal {
	RunVal::Index(build_state(eval_exp(exp, ctx)).measure())
}
