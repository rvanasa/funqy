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
		write!(f, "#{}", self.0)
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

type RunValRc = Rc<RunVal>;
#[derive(Clone,Debug,PartialEq)]
pub enum RunVal {
	Data(DataType, usize), // TODO replace cloning with reference
	Tuple(Vec<RunVal>),
	State(State),
	Func(Pat, Exp),
	Macro(Macro),
	Error(Error),
}

impl fmt::Display for RunVal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&RunVal::Data(ref dt, ref index) => write!(f, "{}", dt.variants[*index]),
			&RunVal::Tuple(ref vals) => write!(f, "({})", vals.iter().map(|val| format!("{}", val)).collect::<Vec<String>>().join(", ")),
			&RunVal::Func(ref pat, ref body) => write!(f, "{:?} -> {:?}", pat, body),
			&RunVal::Macro(ref mc) => write!(f, "{:?}", mc),
			&RunVal::State(ref state) => write!(f, "{}", StateView(state)),
			&RunVal::Error(ref err) => write!(f, "<<{}>>", err),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub struct DataType {
	pub variants: Vec<Ident>,
}

#[derive(Clone,Debug)]
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
		&Exp::Lambda(ref pat, ref body) => RunVal::Func(pat.clone(), (**body).clone()),
		&Exp::Invoke(ref target, ref arg) => {
			match eval_exp(target, ctx) {
				RunVal::Func(pat, body) => {
					let mut child = ctx.create_child();
					assign_pat(&pat, &eval_exp(arg, ctx), &mut child).unwrap();
					eval_exp(&body, &child)
				},
				RunVal::Macro(Macro(_, handle)) => handle(arg, ctx),
				val => RunVal::Error(format!("Cannot invoke {}", val)),
			}
		},
		&Exp::State(ref arg) => RunVal::State(build_state(eval_exp(arg, ctx))),
		&Exp::Extract(ref arg, ref cases) => {
			let state = build_state(eval_exp(arg, ctx));
			let def: State = match cases.get(cases.len() - 1) {
				Some(&Case::Default(ref result)) => build_state(eval_exp(result, ctx)),
				_ => vec![],
			};
			let mut dims: Vec<State> = vec![];
			while dims.len() < state.len() {
				dims.push(def.clone());
			}
			for (i, case) in cases.iter().rev().enumerate() {
				match case {
					&Case::Exp(ref selector, ref result) => {
						let state = build_state(eval_exp(selector, ctx));
						let result_state = build_state(eval_exp(result, ctx));
						for (i, s) in state.iter().enumerate() {
							if !::num::Zero::is_zero(s) {
								dims[i] = result_state.clone();
							}
						}
					},
					&Case::Default(_) => {},
				}
			}
			RunVal::State(state.extract(dims))
		},
		&Exp::Measure(ref arg) => match eval_exp(arg, ctx) {
			RunVal::State(ref state) => RunVal::State(get_state(state.measure())),
			val => val,
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
					assign_pat(pat, val, ctx);
				}
				Ok(())
			}
		},
		_ => Err(format!("{:?} cannot deconstruct `{}`", pat, val))
	}
}

// pub fn resolve_index(val: &Pat) -> Option<(usize, usize)> {
// 	match val {
// 		&RunVal::Data(ref dt, ref index) => Some((dt.variants.len(), *index)),
// 		&RunVal::Tuple(ref vals) => {
// 			let mut acc = (1, 0);
// 			for val in vals {
// 				match resolve_index(val) {
// 					Some((s2, i2)) => {
// 						let (s1, i1) = acc;
// 						acc = (s1 * i1, s2 * i1 + i2);
// 					},
// 					None => return None,
// 				}
// 			}
// 			Some(acc)
// 		},
// 		_ => None,
// 	}
// }

pub fn build_state(val: RunVal) -> State {
	match val {
		RunVal::Data(dt, index) => get_state(index).pad(dt.variants.len()),
		RunVal::Tuple(vals) => vals.into_iter().fold(get_state(0), |a, b| a.combine(build_state(b))),
		RunVal::Func(_pat, _body) => unimplemented!(),
		RunVal::Macro(_mc) => unimplemented!(),
		RunVal::State(state) => state,
		RunVal::Error(err) => panic!(err),
	}
}