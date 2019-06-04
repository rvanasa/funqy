use error::*;
use ast::*;
use engine::*;
use types::*;
use eval_static::*;

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
	Func(Rc<Context>, Pat, Exp, Type),
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
			&RunVal::Func(ref _ctx, ref _pat, ref _body, ref ty) => write!(f, "fn{}", ty),
			&RunVal::Macro(ref mc) => write!(f, "{:?}", mc),
			&RunVal::State(ref state, ref ty) => if ty != &Type::Any {
				write!(f, "{}: {}", StateView(state), ty)
			} else {
				write!(f, "{}", StateView(state))
			},
			&RunVal::Gate(ref gate) => write!(f, "[{}]", gate.iter().map(|state| format!("{}", StateView(state))).collect::<Vec<_>>().join(", ")),
		}
	}
}

#[derive(Clone,Debug,PartialEq)]
pub struct Context {
	path: String,
	vars: HashMap<Ident, RunVal>,
	types: TypeContext,
}

impl Context {
	pub fn new(path: String) -> Context {
		Context {
			path,
			vars: HashMap::new(),
			types: TypeContext::new(),
		}
	}
	
	pub fn path(&self) -> &String {
		&self.path
	}
	
	pub fn types(&self) -> &TypeContext {
		&self.types
	}
	
	pub fn create_child(&self) -> Context {
		self.clone()
	}
	
	pub fn find_var(&self, id: &Ident) -> Ret<RunVal> {
		unwrap_from_context("Variable", id, self.vars.get(id))
	}
	
	pub fn add_var(&mut self, id: Ident, val: RunVal, ty: Type) -> Ret {
		self.vars.insert(id.clone(), val);
		self.types.add_var_type(id, ty)
	}
	
	pub fn find_type(&self, id: &Ident) -> Ret<Type> {
		self.types.find_type(id)
	}
	
	pub fn add_type(&mut self, id: String, ty: Type) -> Ret {
		self.types.add_type(id, ty)
	}
	
	pub fn add_datatype(&mut self, id: String, variants: Vec<Ident>) -> Ret {
		let rc = Rc::new(DataType {id: id.clone(), variants: variants.clone()});
		for (i, variant) in variants.iter().enumerate() {
			self.add_var(variant.clone(), RunVal::Data(rc.clone(), i), Type::Data(rc.clone()))?;
		}
		self.add_type(id, Type::Data(rc))
	}
	
	pub fn add_macro(&mut self, id: &str, handle: &'static Fn(&Exp, &Context) -> Ret<RunVal>) -> Ret {
		self.add_var(id.to_string(), RunVal::Macro(Macro(id.to_string(), Rc::new(handle))), Type::Any /* TODO define macro types */)
	}
	
	pub fn import(&self, path: &str) -> Ret<Module> {
		use regex::Regex;
		use std::path::Path;
		use resource;
		use stdlib;
		use parser;
		
		let (ctx, file) = if Regex::new("^[a-z]+:").unwrap().is_match(path) {(self.create_child(), path.to_string())}
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
		&Exp::Concat(ref args) => {
			//TODO adjacent gates
			if args.len() == 1 {
				if let Some(gate) = build_gate(&eval_exp(&args[0], ctx), ctx) {
					return RunVal::Gate(gate)
				}
			}
			let div = (args.len() as f32).sqrt();
			let states = args.iter()
				.map(|e| build_state_typed(eval_exp(e, ctx)))
				.collect::<Ret<Vec<(State, Type)>>>().unwrap();
			RunVal::State(states.iter()
				.flat_map(|(s, _)| s)
				.map(|n| n / div)
				.collect(),
				Type::Concat(states.into_iter()
					.map(|(_, t)| t)
					.collect()))
		},
		&Exp::Cond(ref cond_exp, ref then_exp, ref else_exp) => {
			let val = eval_exp(cond_exp, ctx);
			if let Some(b) = build_bool(&val) {
				eval_exp(if b {then_exp} else {else_exp}, ctx)
			}
			else {
				// TODO: consider removing in favor of using extract gates for explicitness
//				let state = build_state(val);
//				if state.len() > 2 {
//					panic!("Conditional state cannot be {}-dimensional", state.len())
//				}
//				RunVal::State(state.extract(vec![
//					build_state(eval_exp(else_exp, ctx)),
//					build_state(eval_exp(then_exp, ctx)),
//				]), Type::Any /* TODO determine from then/else types */)
                panic!("Non-boolean value: {}", val)
			}
		},
		&Exp::Lambda(ref pat, ref body) => {
			let ty = infer_type(exp, ctx.types()).unwrap();
			RunVal::Func(Rc::new(ctx.clone()), pat.clone(), (**body).clone(), ty)
		},
		&Exp::Invoke(ref target, ref arg) => {
			match eval_exp(target, ctx) {
				// TODO proper tuple function evaluation
				RunVal::Func(fn_ctx_rc, pat, body, _ty) => {
					let mut fn_ctx = (*fn_ctx_rc).clone();
					assign_pat(&pat, &eval_exp(arg, ctx), &mut fn_ctx).unwrap();
					eval_exp(&body, &fn_ctx)
				},
				RunVal::Macro(Macro(_, handle)) => handle(arg, ctx).unwrap(),
				RunVal::Gate(gate) => {
					let (s, t) = build_state_typed(eval_exp(arg, ctx)).unwrap();
					RunVal::State(s.extract(gate), t)
				},
				val => {
					let msg = &format!("Cannot invoke {}", val);
					let state = build_state(eval_exp(arg, ctx));
					let gate = build_gate(&val, ctx).expect(msg);
					RunVal::State(state.extract(gate), Type::Any /* TODO infer output type from `target` */)
				},
			}
		},
		&Exp::Repeat(n, ref exp) => {
			let val = eval_exp(&exp, ctx);
			RunVal::Tuple((0..n).map(|_| val.clone()).collect())
		},
		&Exp::State(ref arg) => {
			let (s, t) = build_state_typed(eval_exp(arg, ctx)).unwrap();
			RunVal::State(s, t)
		},
		&Exp::Phase(phase, ref arg) => {
			let val = eval_exp(arg, ctx);
			build_gate(&val, ctx)
				.map(|g| RunVal::Gate(g.power(phase)))
				.unwrap_or_else(|| {
					let (s, t) = build_state_typed(val).unwrap();
					RunVal::State(s.phase(phase), t)
				})
		},
		&Exp::Extract(ref arg, ref cases) => {
            let state = build_state(eval_exp(arg, ctx));
            let (gate, gt) = create_extract_gate_typed(cases, state.len(), ctx);
            RunVal::State(state.extract(gate), gt)
		},
		&Exp::Anno(ref exp, ref anno) => eval_type(anno, ctx.types()).unwrap().assign(eval_exp(exp, ctx)).unwrap(),
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
            let val = eval_exp(e, ctx);
            let err = Error(format!("Cannot expand value: {}", val));
            iterate_val(val).ok_or(err).unwrap()
		}
		else {vec![eval_exp(e, ctx)]}
	}).collect()
}

pub fn eval_decl(decl: &Decl, ctx: &mut Context) -> Ret {
	match decl {
		&Decl::Let(ref pat, ref exp) => assign_pat(pat, &eval_exp(exp, ctx), ctx),
		&Decl::Type(ref id, ref pat) => {
			let ty = eval_type(pat, ctx.types())?;
			ctx.add_type(id.clone(), ty)
		},
		&Decl::Data(ref id, ref variants) => ctx.add_datatype(id.clone(), variants.clone()),
		&Decl::Assert(ref expect, ref result) => {
			let a = eval_exp(expect, ctx);
			let b = eval_exp(result, ctx);
			let eq = match (&a, &b) {
				(&RunVal::State(ref a, _), &RunVal::State(ref b, _)) => {
					a.iter().zip(b).map(|(a, b)| {
						let abs = (a - b).norm();
						abs * abs
					}).sum::<f32>() < 0.00001_f32
				},
				(a, b) => a == b,
			};
			if !eq {err!("Assertion failed: {} != {}", a, b)}
			else {Ok(())}
		},
		&Decl::Print(ref exp) => Ok(println!(":: {}", eval_exp(exp, ctx))),
		&Decl::Do(ref exp) => {
			eval_exp(exp, ctx);
			Ok(())
		},
	}
}

// TODO combine logic with eval_static::assign_pat_type()
pub fn assign_pat(pat: &Pat, val: &RunVal, ctx: &mut Context) -> Ret {
	match (pat, val) {
		(&Pat::Any, _) => Ok(()),
		(&Pat::Var(ref id), _) => ctx.add_var(id.clone(), val.clone(), get_val_type(val)), //TODO use val type
		(&Pat::Tuple(ref pats), &RunVal::Tuple(ref vals)) => {
			if pats.len() != vals.len() {err!("Cannot deconstruct {} values from value: {}", pats.len(), val)}
			else {
				pats.iter().zip(vals)
					.map(|(pat, val)| assign_pat(pat, val, ctx))
					.collect::<Ret<_>>()
			}
		},
		(&Pat::Anno(ref pat, ref anno), _) => assign_pat(pat, &eval_type(&anno, ctx.types())?.assign(val.clone())?, ctx),
		_ => err!("{:?} cannot deconstruct `{}`", pat, val),
	}
}

pub fn get_val_type(val: &RunVal) -> Type {
	match val {
		&RunVal::Index(_) => Type::Any,
		&RunVal::String(_) => Type::Any,
		&RunVal::Data(ref dt, _) => Type::Data((*dt).clone()),
		&RunVal::Tuple(ref vals) => Type::Tuple(vals.iter().map(get_val_type).collect()),
		&RunVal::Func(_, _, _, ref ty) => ty.clone(),
		&RunVal::Macro(_) => Type::Any, // TODO
		&RunVal::State(_, ref ty) => ty.clone(),
		&RunVal::Gate(_) => Type::Any, // TODO
	}
}

pub fn build_bool(val: &RunVal) -> Option<bool> {
	match val {
		&RunVal::Index(n) => Some(n > 0),
        &RunVal::Data(ref _ty, n) => Some(n > 0),
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
		RunVal::Tuple(vals) => {
			let states = vals.into_iter().map(|v| build_state_typed(v)).collect::<Ret<Vec<(State, Type)>>>()?;
			let ty = Type::Tuple(states.iter().map(|(_, t)| t.clone()).collect());
			Ok((states.into_iter().fold(get_state(0), |a, (b, _)| State::combine(a, b)), ty))
		},
		RunVal::State(state, ty) => Ok((state, ty)),
		val => err!("Cannot build state from {}", val)
	}
}

pub fn eval_gate_body(exp: &Exp, ctx: &Context) -> Option<Gate> {
	match exp {
		&Exp::Extract(ref _arg, ref cases) => Some(create_extract_gate_typed(cases, 0, ctx).0),
		_ => None,
	}
}

pub fn build_gate(val: &RunVal, ctx: &Context) -> Option<Gate> {
	match val {
		&RunVal::Tuple(ref vals) => vals.iter()
			.fold(Some(vec![get_state(0)]), 
				|a, b| a.and_then(|a| build_gate(b, ctx).map(|b| a.combine(b)))),
		&RunVal::Func(ref fn_ctx, ref _pat, ref body, ref _ty) => eval_gate_body(body, fn_ctx), // TODO use type
		&RunVal::Gate(ref gate) => Some(gate.clone()),
		_ => None,
	}
}

pub fn iterate_val(val: RunVal) -> Option<Vec<RunVal>> {
	match val {
		RunVal::Index(i) => {
			Some((0..i).map(RunVal::Index).collect())
		},
		RunVal::Tuple(vals) => Some(vals),
		_ => None,
	}
}

pub fn create_extract_gate_typed(cases: &Vec<Case>, min_input_size: usize, ctx: &Context) -> (Gate, Type) {
	fn reduce_type(output_type: Option<Type>, t: Type) -> Option<Type> {
		Some(match output_type {
			None => t,
			Some(ot) => if ot == t {t} else {Type::Any},
		})
	}
	let mut dims: Gate = vec![];
	let mut output_type = None;
	for case in cases.iter() {
		match case {
			&Case::Exp(ref selector, ref result) => {
				let selector_state = build_state(eval_exp(selector, ctx));
				let (result_state, result_type) = build_state_typed(eval_exp(result, ctx)).unwrap();
				while dims.len() < selector_state.len() || dims.len() < min_input_size {
					dims.push(vec![]);
				}
				for (i, s) in selector_state.iter().enumerate() {
					let len = ::std::cmp::max(result_state.len(), dims[i].len());
					// TODO improve impl
					dims[i] = result_state.clone().pad(len).into_iter()
						.zip(dims[i].clone().pad(len).into_iter())
						.map(|(r, d)| r * s + d)
						.collect();
				}
				output_type = reduce_type(output_type, result_type);
			},
			&Case::Default(ref result) => {
				let (state, result_type) = build_state_typed(eval_exp(result, ctx)).unwrap();
				for i in 0..dims.len() {
					use num::Zero;
					if dims[i].prob_sum().is_zero() {
						dims[i] = state.clone();
					}
				}
				output_type = reduce_type(output_type, result_type);
			},
		}
	}
	let max_len = dims.iter().map(Vec::len).max().unwrap_or(0);
	let gate: Gate = dims.into_iter().map(|s| s.pad(max_len)).collect();
	// if !gate.is_unitary() {
	// 	panic!("Non-unitary extraction: {:?}", cases);
	// }
	(gate, output_type.unwrap_or(Type::Any))
}
