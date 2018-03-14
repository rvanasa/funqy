use ast::*;
use engine::*;

use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

pub type Error = String;

#[derive(Clone)]
pub struct Macro(pub Ident, pub Rc<Fn(&Exp, &Context) -> RunVal>);

impl fmt::Debug for Macro {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, ":macro: {}", self.0)
	}
}

impl PartialEq for Macro {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
	fn ne(&self, other: &Self) -> bool {
		self.0 != other.0
	}
}

#[derive(Clone,Debug,PartialEq)]
pub enum RunVal {
	Index(usize),
	Data(DataType, usize), // TODO replace cloning with reference
	Tuple(Vec<RunVal>),
	Func(Rc<Context>, Pat, Exp),
	Macro(Macro),
	State(State),
	Gate(Gate),
	Error(Error),
}

impl fmt::Display for RunVal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&RunVal::Index(ref n) => write!(f, "{}", n),
			&RunVal::Data(ref dt, ref index) => write!(f, "{}", dt.variants[*index]),
			&RunVal::Tuple(ref vals) => write!(f, "({})", vals.iter().map(|val| format!("{}", val)).collect::<Vec<String>>().join(", ")),
			&RunVal::Func(ref _ctx, ref _pat, ref _body) => write!(f, "(..) -> (..)"),
			&RunVal::Macro(ref mc) => write!(f, "{:?}", mc),
			&RunVal::State(ref state) => write!(f, "{}", StateView(state)),
			&RunVal::Gate(ref gate) => write!(f, "[{}]", gate.iter().map(|state| format!("{}", StateView(state))).collect::<Vec<String>>().join(", ")),
			&RunVal::Error(ref err) => write!(f, "<<{}>>", err),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub struct DataType {
	pub variants: Vec<Ident>,
}

#[derive(Clone,Debug,PartialEq)]
pub struct Context {
	vars: HashMap<Ident, RunVal>,
	datatypes: HashMap<Ident, DataType>,
}

impl Context {
	pub fn new() -> Context {
		Context {
			vars: HashMap::new(),
			datatypes: HashMap::new(),
		}
	}
	
	pub fn create_child(&self) -> Context {
		self.clone()
	}
	
	pub fn add_var(&mut self, id: Ident, val: RunVal) {
		self.vars.insert(id, val);
	}
	
	pub fn find_var(&self, id: &Ident) -> RunVal {
		unwrap("Variable", id, self.vars.get(id))
	}
	
	pub fn add_data(&mut self, id: Ident, dt: DataType) {
		self.datatypes.insert(id, dt.clone());
		for (i, variant) in dt.variants.iter().enumerate() {
			self.add_var(variant.clone(), RunVal::Data(dt.clone(), i));
		}
	}
	
	pub fn find_data(&self, id: &Ident) -> DataType {
		unwrap("Data value", id, self.datatypes.get(id))
	}
	
	pub fn add_macro(&mut self, id: &str, handle: &'static Fn(&Exp, &Context) -> RunVal) {
		self.add_var(id.to_string(), RunVal::Macro(Macro(id.to_string(), Rc::new(handle))))
	}
}

fn unwrap<T:Clone>(cat: &str, id: &Ident, opt: Option<&T>) -> T {
	(*opt.expect(&format!("{} not found in scope: `{}`", cat, id))).clone()
}

pub fn eval_exp(exp: &Exp, ctx: &Context) -> RunVal {
	match exp {
		&Exp::Literal(n) => RunVal::Index(n),
		&Exp::Var(ref id) => ctx.find_var(id),
		&Exp::Scope(ref decls, ref ret) => {
			let mut child = ctx.create_child();
			for decl in decls {
				eval_decl(decl, &mut child);
			}
			eval_exp(ret, &child)
		},
		&Exp::Tuple(ref args) => RunVal::Tuple(args.iter()
			.map(|arg| eval_exp(arg, ctx))
			.collect()),
		&Exp::Lambda(ref pat, ref body) => {
			let fn_ctx = ctx.create_child(); // TODO optimize?
			RunVal::Func(Rc::new(fn_ctx), pat.clone(), (**body).clone())
		},
		&Exp::Invoke(ref target, ref arg) => {
			match eval_exp(target, ctx) {
				RunVal::Tuple(fns) => {
					let state = build_state(eval_exp(arg, ctx));
					let gate = fns.into_iter().map(|f| eval_gate(f, ctx)).fold(vec![get_state(0)], |a, b| a.combine(b));
					RunVal::State(state.extract(gate))
				},
				RunVal::Func(fn_ctx_rc, pat, body) => {
					let mut fn_ctx = (*fn_ctx_rc).clone();
					assign_pat(&pat, &eval_exp(arg, ctx), &mut fn_ctx).unwrap();
					eval_exp(&body, &fn_ctx)
				},
				RunVal::Macro(Macro(_, handle)) => handle(arg, ctx),
				RunVal::Gate(gate) => RunVal::State(build_state(eval_exp(arg, ctx)).extract(gate)),
				val => RunVal::Error(format!("Cannot invoke {}", val)),
			}
		},
		&Exp::State(ref arg) => RunVal::State(build_state(eval_exp(arg, ctx))),
		&Exp::Phase(phase, ref arg) => RunVal::State(build_state(eval_exp(arg, ctx)).phase(phase)),
		&Exp::Extract(ref arg, ref cases) => {
			let state = build_state(eval_exp(arg, ctx));
			let gate = extract_gate(cases, ctx);
			RunVal::State(state.extract(gate))
		},
	}
}

pub fn eval_decl(decl: &Decl, ctx: &mut Context) {
	match decl {
		&Decl::Data(ref id, ref variants) => {
			let dt = DataType {variants: variants.clone()};
			ctx.add_data(id.clone(), dt);
		},
		&Decl::Let(ref pat, ref exp) => assign_pat(pat, &eval_exp(exp, ctx), ctx).unwrap(),
		&Decl::Assert(ref expect, ref result) => {
			let a = eval_exp(expect, ctx);
			let b = eval_exp(result, ctx);
			if a != b {
				panic!("Assertion failed: {} != {}", a, b);
			}
		},
		&Decl::Print(ref exp) => println!(":: {}", eval_exp(exp, ctx)),
	}
}

pub fn assign_pat(pat: &Pat, val: &RunVal, ctx: &mut Context) -> Result<(), Error> {
	match (pat, val) {
		(&Pat::Wildcard, _) => {Ok(())},
		(&Pat::Var(ref id), _) => {ctx.add_var(id.clone(), val.clone()); Ok(())},
		(&Pat::Tuple(ref pats), &RunVal::Tuple(ref vals)) => {
			if pats.len() != vals.len() {Err(format!("Cannot deconstruct {} values from value: {}", pats.len(), val))}
			else {
				for (pat, val) in pats.iter().zip(vals) {
					match assign_pat(pat, val, ctx) {
						Ok(()) => {},
						Err(err) => return Err(err),
					}
				}
				Ok(())
			}
		},
		_ => Err(format!("{:?} cannot deconstruct `{}`", pat, val))
	}
}

pub fn build_state(val: RunVal) -> State {
	match val {
		RunVal::Index(n) => get_state(n),
		RunVal::Data(dt, index) => get_state(index).pad(dt.variants.len()),
		RunVal::Tuple(vals) => vals.into_iter().fold(get_state(0), |a, b| a.combine(build_state(b))),
		RunVal::Func(_ctx, _pat, _body) => unimplemented!(),
		RunVal::Macro(_mc) => unimplemented!(),
		RunVal::State(state) => state,
		RunVal::Gate(_state) => unimplemented!(),
		RunVal::Error(err) => panic!(err),
	}
}

pub fn eval_gate_body(exp: &Exp, ctx: &Context) -> Gate {
	match exp {
		&Exp::Extract(ref _arg, ref cases) => {
			extract_gate(cases, ctx)
		},
		// _ => eval_val_gate(eval_exp(exp, ctx), ctx)
		_ => unimplemented!(),
	}
}

pub fn eval_gate(val: RunVal, ctx: &Context) -> Gate {
	match val {
		RunVal::Tuple(_vals) => unimplemented!(),
		RunVal::Func(fn_ctx, _pat, body) => {
			eval_gate_body(&body, &fn_ctx)
		},
		RunVal::Gate(gate) => gate,
		_ => panic!(format!("Not a gate: {}", val)),
	}
}

pub fn extract_gate(cases: &Vec<Case>, ctx: &Context) -> Gate {
	let mut dims: Gate = vec![];
	for case in cases.iter() {
		match case {
			&Case::Exp(ref selector, ref result) => {
				let selector_state = build_state(eval_exp(selector, ctx));
				let result_state = build_state(eval_exp(result, ctx));
				while dims.len() < selector_state.len() {
					dims.push(vec![]);
				}
				for (i, s) in selector_state.iter().enumerate() {
					let len = ::std::cmp::max(result_state.len(), dims[i].len());
					// TODO improve impl
					dims[i] = result_state.clone().pad(len).into_iter().zip(dims[i].clone().pad(len).into_iter()).map(|(r, d)| r * s + d).collect();
				}
			},
			&Case::Default(ref result) => {
				let state = build_state(eval_exp(result, ctx));
				for i in 0..dims.len() {
					use num::Zero;
					if dims[i].prob_sum().is_zero() {
						dims[i] = state.clone();
					}
				}
			},
		}
	}
	//??
	let max_len = dims.iter().map(|s| s.len()).max().unwrap_or_else(|| 0);
	dims.into_iter().map(|s| s.pad(max_len)).collect()
}