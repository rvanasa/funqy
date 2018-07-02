use error::*;
use ast::*;
use engine::*;
use types::*;

use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

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
	String(String),
	Data(Rc<DataType>, usize),
	Tuple(Vec<RunVal>),
	Func(Rc<Context>, Pat, Exp),
	Macro(Macro),
	State(State),
	Gate(Gate),
}

impl fmt::Display for RunVal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&RunVal::Index(ref n) => write!(f, "{}", n),
			&RunVal::String(ref s) => write!(f, "{:?}", s),
			&RunVal::Data(ref dt, ref index) => write!(f, "{}", dt.variants[*index]),
			&RunVal::Tuple(ref vals) => write!(f, "({})", vals.iter().map(|val| format!("{}", val)).collect::<Vec<_>>().join(", ")),
			&RunVal::Func(ref _ctx, ref _pat, ref _body) => write!(f, "(..) -> (..)"),
			&RunVal::Macro(ref mc) => write!(f, "{:?}", mc),
			&RunVal::State(ref state) => write!(f, "{}", StateView(state)),
			&RunVal::Gate(ref gate) => write!(f, "[{}]", gate.iter().map(|state| format!("{}", StateView(state))).collect::<Vec<_>>().join(", ")),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub struct Context {
	path: String,
	vars: HashMap<Ident, RunVal>,
	types: HashMap<Ident, Type>,
}

impl Context {
	pub fn new(path: String) -> Context {
		Context {
			path: path,
			vars: HashMap::new(),
			types: HashMap::new(),
		}
	}
	
	pub fn path<'a>(&'a self) -> &'a String {
		&self.path
	}
	
	pub fn create_child(&self) -> Context {
		self.clone()
	}
	
	pub fn find_var(&self, id: &Ident) -> Ret<RunVal> {
		find("Variable", id, self.vars.get(id))
	}
	
	pub fn add_var(&mut self, id: Ident, val: RunVal) {
		self.vars.insert(id, val);
	}
	
	pub fn find_type(&self, id: &Ident) -> Ret<Type> {
		find("Type", id, self.types.get(id))
	}
	
	pub fn add_type(&mut self, id: Ident, ty: Type) {
		self.types.insert(id, ty);
	}
	
	pub fn add_datatype(&mut self, id: String, variants: Vec<Ident>) {
		let rc = Rc::new(DataType {id: id.clone(), variants: variants.clone()});
		for (i, variant) in variants.iter().enumerate() {
			self.add_var(variant.clone(), RunVal::Data(rc.clone(), i));
		}
		self.add_type(id, Type::Data(rc));
	}
	
	pub fn add_macro(&mut self, id: &str, handle: &'static Fn(&Exp, &Context) -> RunVal) {
		self.add_var(id.to_string(), RunVal::Macro(Macro(id.to_string(), Rc::new(handle))))
	}
	
	pub fn import(&self, path: &str) -> Module {
		use std::path::Path;
		use resource;
		use stdlib;
		use parser;
		
		let import_path = Path::new(&self.path()).join(&resource::with_ext(path, "fqy"));
		let mut import_dir = import_path.clone();
		import_dir.pop();
		let ctx = stdlib::create_ctx(import_dir.to_str().unwrap());
		let file = format!("{}", import_path.to_string_lossy());
		let exp = parser::parse_resource(&file).expect("Failed to parse imported script");
		Module {path: file, exp: exp, ctx: ctx}
	}
	
	pub fn import_eval(&self, path: &str) -> RunVal {
		let mut module = self.import(path);
		eval_exp_inline(&module.exp, &mut module.ctx)
	}
}

#[derive(Clone,Debug,PartialEq)]
pub struct Module {
	pub path: String,
	pub exp: Exp,
	pub ctx: Context,
}

fn find<T:Clone>(cat: &str, id: &Ident, opt: Option<&T>) -> Ret<T> {
	opt.map(|t| t.clone()).ok_or_else(|| Error(format!("{} not found in scope: `{}`", cat, id)))
}

pub fn eval_exp(exp: &Exp, ctx: &Context) -> RunVal {
	match exp {
		&Exp::Index(n) => RunVal::Index(n),
		&Exp::String(ref s) => RunVal::String(s.to_string()),
		&Exp::Var(ref id) => ctx.find_var(id).unwrap(),
		&Exp::Scope(ref decls, ref ret) => {
			let mut child = ctx.create_child();
			for decl in decls {
				eval_decl(decl, &mut child);
			}
			eval_exp(ret, &child)
		},
		&Exp::Expand(_) => panic!("No context for expansion"),
		&Exp::Tuple(ref args) => RunVal::Tuple(eval_exp_seq(args, ctx)),
		&Exp::Concat(ref args) => RunVal::State(args.iter()
			.flat_map(|e| build_state(eval_exp(e, ctx)))
			.collect::<State>().normalized()), // TODO gates
		&Exp::Cond(ref cond_exp, ref then_exp, ref else_exp) => {
			let val = eval_exp(cond_exp, ctx);
			if let Some(b) = build_bool(&val) {
				eval_exp(if b {then_exp} else {else_exp}, ctx)
			}
			else {
				let state = build_state(val);
				if state.len() > 2 {
					panic!("Conditional state canot be {}-dimensional", state.len())
				}
				RunVal::State(state.extract(vec![
					build_state(eval_exp(else_exp, ctx)),
					build_state(eval_exp(then_exp, ctx)),
				]))
			}
		},
		&Exp::Lambda(ref pat, ref body) => {
			let fn_ctx = ctx.create_child(); // TODO optimize?
			RunVal::Func(Rc::new(fn_ctx), pat.clone(), (**body).clone())
		},
		&Exp::Invoke(ref target, ref arg) => {
			match eval_exp(target, ctx) {
				// TODO proper tuple function evaluation
				RunVal::Func(fn_ctx_rc, pat, body) => {
					let mut fn_ctx = (*fn_ctx_rc).clone();
					assign_pat(&pat, &eval_exp(arg, ctx), &mut fn_ctx).unwrap();
					eval_exp(&body, &fn_ctx)
				},
				RunVal::Macro(Macro(_, handle)) => handle(arg, ctx),
				RunVal::Gate(gate) => RunVal::State(build_state(eval_exp(arg, ctx)).extract(gate)),
				val => {
					let msg = &format!("Cannot invoke {}", val)[..];
					let state = build_state(eval_exp(arg, ctx));
					let gate = build_gate(&val, ctx).expect(msg);
					RunVal::State(state.extract(gate))
				},
			}
		},
		&Exp::State(ref arg) => RunVal::State(build_state(eval_exp(arg, ctx))),
		&Exp::Phase(phase, ref arg) => {
			let val = eval_exp(arg, ctx);
			build_gate(&val, ctx)
				.map(|g| RunVal::Gate(g.power(phase)))
				.unwrap_or_else(|| RunVal::State(build_state(val).phase(phase)))
		},
		&Exp::Extract(ref arg, ref cases) => {
			let state = build_state(eval_exp(arg, ctx));
			let gate = create_extract_gate(cases, ctx);
			RunVal::State(state.extract(gate))
		},
		&Exp::Anno(ref exp, ref anno) => eval_type(anno, ctx).unwrap().assign(eval_exp(exp, ctx)).unwrap(),
	}
}

pub fn eval_exp_inline(exp: &Exp, ctx: &mut Context) -> RunVal {
	match exp {
		Exp::Scope(ref decls, ref exp) => {
			for decl in decls {
				eval_decl(decl, ctx);
			}
			eval_exp(exp, ctx)
		},
		_ => eval_exp(&exp, ctx),
	}
}

pub fn eval_exp_seq(seq: &Vec<Exp>, ctx: &Context) -> Vec<RunVal> {
	seq.iter().flat_map(|e| {
		if let Exp::Expand(ref e) = e {
			match eval_exp(e, ctx) {
				RunVal::Tuple(args) => args,
				_ => panic!("Cannot expand value")
			}
		}
		else {vec![eval_exp(e, ctx)]}
	}).collect()
}

pub fn eval_type(pat: &Pat, ctx: &Context) -> Ret<Type> {
	match pat {
		&Pat::Any => Ok(Type::Any),
		&Pat::Var(ref id) => ctx.find_type(id),
		&Pat::Tuple(ref args) => args.iter()
			.map(|p| eval_type(p, ctx))
			.collect::<Ret<_>>()
			.map(Type::Tuple),
		&Pat::Anno(_, _) => Err(Error(format!("Annotations not allowed in types"))),
	}
}

pub fn eval_decl(decl: &Decl, ctx: &mut Context) {
	match decl {
		&Decl::Data(ref id, ref variants) => ctx.add_datatype(id.clone(), variants.clone()),
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

pub fn assign_pat(pat: &Pat, val: &RunVal, ctx: &mut Context) -> Ret {
	match (pat, val) {
		(&Pat::Any, _) => Ok(()),
		(&Pat::Var(ref id), _) => {ctx.add_var(id.clone(), val.clone()); Ok(())},
		(&Pat::Tuple(ref pats), &RunVal::Tuple(ref vals)) => {
			if pats.len() != vals.len() {Err(Error(format!("Cannot deconstruct {} values from value: {}", pats.len(), val)))}
			else {
				pats.iter().zip(vals)
					.map(|(pat, val)| assign_pat(pat, val, ctx))
					.collect::<Ret<_>>()
			}
		},
		(&Pat::Anno(ref pat, ref anno), val) => assign_pat(pat, &eval_type(&**anno, ctx)?.assign(val.clone())?, ctx),
		_ => Err(Error(format!("{:?} cannot deconstruct `{}`", pat, val))),
	}
}

pub fn build_bool(val: &RunVal) -> Option<bool> {
	match val {
		&RunVal::Index(n) => Some(n > 0),
		&RunVal::Tuple(ref vec) => Some(vec.len() > 0),
		_ => None,
	}
}

pub fn build_state(val: RunVal) -> State {
	match val {
		RunVal::Index(n) => get_state(n),
		RunVal::String(_) => unimplemented!(),
		RunVal::Data(dt, index) => get_state(index).pad(dt.variants.len()),
		RunVal::Tuple(vals) => vals.into_iter().fold(get_state(0), |a, b| a.combine(build_state(b))),
		RunVal::Func(_ctx, _pat, _body) => unimplemented!(),
		RunVal::Macro(_mc) => unimplemented!(),
		RunVal::State(state) => state,
		RunVal::Gate(_gate) => unimplemented!(),
	}
}

pub fn eval_gate_body(exp: &Exp, ctx: &Context) -> Option<Gate> {
	match exp {
		&Exp::Extract(ref _arg, ref cases) => Some(create_extract_gate(cases, ctx)),
		_ => None,
	}
}

pub fn build_gate(val: &RunVal, ctx: &Context) -> Option<Gate> {
	match val {
		&RunVal::Tuple(ref vals) => vals.iter()
			.fold(Some(vec![get_state(0)]), 
				|a, b| a.and_then(|a| build_gate(b, ctx).map(|b| a.combine(b)))),
		&RunVal::Func(ref fn_ctx, ref _pat, ref body) => eval_gate_body(body, fn_ctx),
		&RunVal::Gate(ref gate) => Some(gate.clone()),
		_ => None,
	}
}

pub fn create_extract_gate(cases: &Vec<Case>, ctx: &Context) -> Gate {
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
	let max_len = dims.iter().map(Vec::len).max().unwrap_or(0);
	let gate: Gate = dims.into_iter().map(|s| s.pad(max_len)).collect();
	// if !gate.is_unitary() {
	// 	panic!("Non-unitary extraction: {:?}", cases);
	// }
	gate
}