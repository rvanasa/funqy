use error::*;
use ast::*;
use engine::*;
use types::*;

use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Macro(pub Ident, pub Rc<Fn(&Exp, &Context) -> Ret<RunVal>>);

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
	State(State, Type),
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
			&RunVal::State(ref state, ref ty) => if ty != &Type::Any {
				write!(f, "{}: {}", StateView(state), ty)
			} else {write!(f, "{}", StateView(state))},
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
	
	pub fn add_var(&mut self, id: Ident, val: RunVal) -> Ret {
		self.vars.insert(id, val);
		Ok(())
	}
	
	pub fn find_type(&self, id: &Ident) -> Ret<Type> {
		find("Type", id, self.types.get(id))
	}
	
	pub fn add_type(&mut self, id: Ident, ty: Type) -> Ret {
		self.types.insert(id, ty);
		Ok(())
	}
	
	pub fn add_datatype(&mut self, id: String, variants: Vec<Ident>) -> Ret {
		let rc = Rc::new(DataType {id: id.clone(), variants: variants.clone()});
		for (i, variant) in variants.iter().enumerate() {
			self.add_var(variant.clone(), RunVal::Data(rc.clone(), i))?;
		}
		self.add_type(id, Type::Data(rc))
	}
	
	pub fn add_macro(&mut self, id: &str, handle: &'static Fn(&Exp, &Context) -> Ret<RunVal>) -> Ret {
		self.add_var(id.to_string(), RunVal::Macro(Macro(id.to_string(), Rc::new(handle))))
	}
	
	pub fn import(&self, path: &str) -> Ret<Module> {
		use std::path::Path;
		use resource;
		use stdlib;
		use parser;
		
		let (ctx, file) = if path.starts_with("raw:") {(self.clone(), path.to_string() /*TODO replace with context file*/)}
		else {
			let import_path = Path::new(&self.path()).join(&resource::with_ext(path, "fqy"));
			let mut import_dir = import_path.clone();
			import_dir.pop();
			let file = import_path.to_string_lossy().to_string();
			let ctx = stdlib::create_ctx(&import_dir.to_string_lossy())?;
			(ctx, file)
		};
		let exp = parser::parse_resource(&file)?;
		Ok(Module {path: file.to_string(), exp: exp, ctx: ctx})
	}
	
	pub fn import_eval(&self, path: &str) -> Ret<RunVal> {
		let mut module = self.import(path)?;
		Ok(eval_exp_inline(&module.exp, &mut module.ctx))
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
				eval_decl(decl, &mut child).unwrap();
			}
			eval_exp(ret, &child)
		},
		&Exp::Expand(_) => panic!("No context for expansion"),
		&Exp::Tuple(ref args) => RunVal::Tuple(eval_exp_seq(args, ctx)),
		&Exp::Concat(ref args) => RunVal::State(args.iter()
			.flat_map(|e| build_state(eval_exp(e, ctx)))
			.collect::<State>().normalized(), // TODO gates
			Type::Any),
		&Exp::Cond(ref cond_exp, ref then_exp, ref else_exp) => {
			let val = eval_exp(cond_exp, ctx);
			if let Some(b) = build_bool(&val) {
				eval_exp(if b {then_exp} else {else_exp}, ctx)
			}
			else {
				let state = build_state(val);
				if state.len() > 2 {
					panic!("Conditional state cannot be {}-dimensional", state.len())
				}
				RunVal::State(state.extract(vec![
					build_state(eval_exp(else_exp, ctx)),
					build_state(eval_exp(then_exp, ctx)),
				]), Type::Any /* TODO sum of then/else types */)
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
				RunVal::Macro(Macro(_, handle)) => handle(arg, ctx).unwrap(),
				RunVal::Gate(gate) => RunVal::State(build_state(eval_exp(arg, ctx)).extract(gate), Type::Any /* TODO maintain arg type */),
				val => {
					let msg = &format!("Cannot invoke {}", val);
					let state = build_state(eval_exp(arg, ctx));
					let gate = build_gate(&val, ctx).expect(msg);
					RunVal::State(state.extract(gate), Type::Any /* TODO maintain arg type */)
				},
			}
		},
		&Exp::Repeat(n, ref exp) => {
			let val = eval_exp(&*exp, ctx);
			RunVal::Tuple((0..n).map(|_| val.clone()).collect())
		},
		&Exp::State(ref arg) => RunVal::State(build_state(eval_exp(arg, ctx)), Type::Any /* TODO maintain arg type */),
		&Exp::Phase(phase, ref arg) => {
			let val = eval_exp(arg, ctx);
			build_gate(&val, ctx)
				.map(|g| RunVal::Gate(g.power(phase)))
				.unwrap_or_else(|| RunVal::State(build_state(val).phase(phase), Type::Any /* TODO maintain val type */))
		},
		&Exp::Extract(ref arg, ref cases) => {
			let state = build_state(eval_exp(arg, ctx));
			let gate = create_extract_gate(cases, ctx);
			RunVal::State(state.extract(gate), Type::Any /* TODO maintain gate type */)
		},
		&Exp::Anno(ref exp, ref anno) => eval_type(anno, ctx).unwrap().assign(eval_exp(exp, ctx)).unwrap(),
	}
}

pub fn eval_exp_inline(exp: &Exp, ctx: &mut Context) -> RunVal {
	match exp {
		Exp::Scope(ref decls, ref exp) => {
			for decl in decls {
				eval_decl(decl, ctx).unwrap();
			}
			eval_exp(exp, ctx)
		},
		_ => eval_exp(exp, ctx),
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
		&Pat::Concat(ref args) => args.iter()
			.map(|p| eval_type(p, ctx))
			.collect::<Ret<_>>()
			.map(Type::Concat),
		&Pat::Anno(_, _) => Err(Error(format!("Annotations not allowed in types"))),
		&Pat::Repeat(n, ref pat) => {
			let ty = eval_type(&*pat, ctx);
			(0..n).map(|_| ty.clone()).collect::<Ret<_>>().map(Type::Tuple)
		},
	}
}

pub fn eval_decl(decl: &Decl, ctx: &mut Context) -> Ret {
	match decl {
		&Decl::Let(ref pat, ref exp) => assign_pat(pat, &eval_exp(exp, ctx), ctx),
		&Decl::Type(ref id, ref pat) => {
			let ty = eval_type(pat, ctx)?;
			ctx.add_type(id.clone(), ty)
		},
		&Decl::Data(ref id, ref variants) => ctx.add_datatype(id.clone(), variants.clone()),
		&Decl::Assert(ref expect, ref result) => {
			let a = eval_exp(expect, ctx);
			let b = eval_exp(result, ctx);
			if a != b {err!("Assertion failed: {} != {}", a, b)}
			else {Ok(())}
		},
		&Decl::Print(ref exp) => Ok(println!(":: {}", eval_exp(exp, ctx))),
		&Decl::Do(ref exp) => {
			eval_exp(exp, ctx);
			Ok(())
		},
	}
}

pub fn assign_pat(pat: &Pat, val: &RunVal, ctx: &mut Context) -> Ret {
	match (pat, val) {
		(&Pat::Any, _) => Ok(()),
		(&Pat::Var(ref id), _) => ctx.add_var(id.clone(), val.clone()),
		(&Pat::Tuple(ref pats), &RunVal::Tuple(ref vals)) => {
			if pats.len() != vals.len() {err!("Cannot deconstruct {} values from value: {}", pats.len(), val)}
			else {
				pats.iter().zip(vals)
					.map(|(pat, val)| assign_pat(pat, val, ctx))
					.collect::<Ret<_>>()
			}
		},
		(&Pat::Concat(ref _pats), &RunVal::State(ref _state, _)) => {
			unimplemented!()
		},
		(&Pat::Anno(ref pat, ref anno), val) => assign_pat(pat, &eval_type(&**anno, ctx)?.assign(val.clone())?, ctx),
		_ => err!("{:?} cannot deconstruct `{}`", pat, val),
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
	build_state_typed(val).unwrap().0
}

pub fn build_state_typed(val: RunVal) -> Ret<(State, Type)> {
	match val {
		RunVal::Index(n) => Ok((get_state(n), Type::Any)),
		RunVal::Data(dt, index) => Ok((get_state(index).pad(dt.variants.len()), Type::Data(dt))),
		RunVal::Tuple(vals) => Ok((vals.into_iter().fold(get_state(0), |a, b| a.combine(build_state(b))), Type::Any /* TODO type from vals */)),
		RunVal::State(state, ty) => Ok((state, ty)),
		val => err!("Cannot build state from {}", val)
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