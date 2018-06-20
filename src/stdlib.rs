use ast::Exp;
use engine::*;
use parser::parse_file;
use eval::*;

use std::path::Path;

pub fn create_ctx(path: &str) -> Context {
	let mut ctx = Context::new(path.to_string());
	ctx.add_macro("import", &lib_import);
	ctx.add_macro("sup", &lib_sup);
	ctx.add_macro("phf", &lib_phf);
	ctx.add_macro("gate", &lib_gate);
	ctx.add_macro("inv", &lib_inv);
	ctx.add_macro("repeat", &lib_repeat);
	ctx.add_macro("measure", &lib_measure);
	ctx
}

fn lib_import(exp: &Exp, ctx: &Context) -> RunVal {
	match eval_exp(exp, ctx) {
		RunVal::String(ref s) => {
			let mut import_path = Path::new(ctx.path().as_str()).join(s.as_str());
			let mut import_dir = import_path.clone();
			import_dir.pop();
			let mut import_ctx = create_ctx(import_dir.to_str().unwrap());
			let mut file = format!("{}", import_path.to_string_lossy());
			if !file.ends_with(".fqy") {
				file = format!("{}.fqy", file);
			}
			let exp = parse_file(file.as_str()).expect("Failed to parse imported script");
			eval_exp_inline(&exp, &mut import_ctx)
		},
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
	RunVal::Gate(build_gate(&eval_exp(exp, ctx), ctx).unwrap().inverse())
}

fn lib_repeat(exp: &Exp, ctx: &Context) -> RunVal {
	fn do_repeat(state: State, n: usize) -> State {
		let div = (n as f32).sqrt();
		(0..n).flat_map(|_| state.iter().map(|s| s / div)).collect()
	}
	match exp {
		&Exp::Tuple(ref args) => {
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
