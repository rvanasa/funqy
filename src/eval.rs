use ast::*;
use engine::*;

use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

type Error = String;

type RunValRc = Rc<RunVal>;
#[derive(Clone,Debug)]
pub enum RunVal {
	Unit,
	Data(DataType, usize), // TODO replace cloning with reference
	Tuple(Vec<RunVal>),
	State(State),
	// Func(Pat, Exp),
	// Unknown,
}

impl fmt::Display for RunVal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&RunVal::Unit => write!(f, "()"),
			&RunVal::Data(_, ref index) => write!(f, "{}", index),
			&RunVal::Tuple(ref vals) => write!(f, "({})", vals.iter().map(move |val| format!("{}", val)).collect::<Vec<String>>().join(", ")),
			&RunVal::State(ref state) => write!(f, "[{}]", state.iter().map(move |index| format!("{}", index)).collect::<Vec<String>>().join(", ")),
		}
	}
}

#[derive(Clone,Debug)]
pub struct DataType {
	pub choices: Vec<Ident>,
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
		for (i, choice) in dt.choices.iter().enumerate() {
			self.add_var(choice, RunVal::Data(dt.clone(), i));
		}
	}
	
	pub fn find_data(&self, id: &Ident) -> DataType {
		unwrap("Data value", id, self.datatypes.get(id))
	}
}

fn unwrap<T:Clone>(cat: &str, id: &Ident, opt: Option<&T>) -> T {
	(*opt.expect(&format!("{} not found in scope: `{}`", cat, id))).clone()
}

pub fn eval_exp(exp: &Exp, ctx: &Context) -> RunVal {
	match exp {
		&Exp::Var(ref id) => ctx.find_var(id),
		&Exp::Scope(ref decls, ref exp) => {
			let mut child = ctx.create_child();
			for decl in decls {
				eval_decl(decl, &mut child);
			}
			eval_exp(exp, &child)
		},
		&Exp::Tuple(ref args) => RunVal::Tuple(args.iter().map(move |arg| eval_exp(arg, ctx)).collect()),
		// &Exp::Data(ref id) => {
		// 	RunVal::Data(ctx.find_data(id))
		// },
		&Exp::State(ref arg) => RunVal::State(build_state(eval_exp(arg, ctx))),
		&Exp::Extract(ref arg, ref dims) => {
			let state = build_state(eval_exp(arg, ctx));
			RunVal::State(state.extract(dims.iter().map(move |dim| build_state(eval_exp(dim, ctx))).collect()))
		},
		&Exp::Sup(ref exp_a, ref exp_b) => {
			let a = build_state(eval_exp(exp_a, ctx));
			let b = build_state(eval_exp(exp_b, ctx));
			RunVal::State(a.sup(b))
		},
		&Exp::Measure(ref arg) => match eval_exp(arg, ctx) {
			RunVal::State(ref state) => RunVal::State(get_state(state.measure())),
			val => val,
		},
	}
}

pub fn eval_decl(decl: &Decl, ctx: &mut Context) {
	match decl {
		&Decl::Data(ref id, ref choices) => {
			let dt = DataType {choices: choices.clone()};
			ctx.add_data(id, dt);
		},
		&Decl::Let(ref pat, ref exp) => match assign_pat(pat, &eval_exp(exp, ctx), ctx) {
			Err(err) => panic!(err),
			_ => {},
		},
	}
}

pub fn assign_pat(pat: &Pat, val: &RunVal, ctx: &mut Context) -> Result<(), Error> {
	match (pat, val) {
		(&Pat::Unit, &RunVal::Unit) => Ok(()),
		(&Pat::Var(ref id), _) => {ctx.add_var(id, val.clone()); Ok(())},
		(&Pat::Tuple(ref pats), &RunVal::Tuple(ref vals)) => {
			if pats.len() != vals.len() {Err(format!("Invalid tuple length"))}
			else {
				for (pat, val) in pats.iter().zip(vals) {
					assign_pat(pat, val, ctx);
				}
				Ok(())
			}
		},
		_ => Err(format!("{:?} cannot deconstruct {:?}", pat, val))
	}
}

pub fn build_state(val: RunVal) -> State {
	match val {
		RunVal::Unit => vec![],
		RunVal::Data(dt, index) => get_state(index).pad(dt.choices.len()),
		RunVal::Tuple(vals) => vals.into_iter().fold(get_state(0), move |a, b| a.combine(build_state(b))),
		RunVal::State(state) => state,
	}
}