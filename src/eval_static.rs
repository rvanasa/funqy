use error::*;
use ast::*;
use types::*;

use std::rc::Rc;
use std::collections::HashMap;

#[derive(Clone,Debug,PartialEq)]
pub struct TypeContext {
	types: HashMap<Ident, Type>,
}

impl TypeContext {
	pub fn new() -> TypeContext {
		TypeContext {
			types: HashMap::new(),
		}
	}
	
	pub fn create_child(&self) -> TypeContext {
		self.clone()
	}
	
	pub fn add_type(&mut self, id: Ident, ty: Type) -> Ret {
		self.types.insert(id, ty);
		Ok(())
	}
	
	pub fn find_type(&self, id: &Ident) -> Ret<Type> {
		unwrap_from_context("Type", id, self.types.get(id))
	}
	
	pub fn add_var_type(&mut self, id: Ident, ty: Type) -> Ret {
		self.types.insert(format!("@{}", id), ty);
		Ok(())
	}
	
	pub fn find_var_type(&self, id: &Ident) -> Ret<Type> {
		unwrap_from_context("Variable type", id, self.types.get(&format!("@{}", id)))
	}
	
	pub fn add_datatype_type(&mut self, id: String, variants: Vec<Ident>) -> Ret {
		let rc = Rc::new(DataType {id: id.clone(), variants: variants.clone()});
		for variant in variants.iter() {
			self.add_var_type(variant.clone(), Type::Data(rc.clone()))?;
		}
		self.add_type(id, Type::Data(rc))
	}
}

pub fn unwrap_from_context<T:Clone>(cat: &str, id: &Ident, opt: Option<&T>) -> Ret<T> {
	opt.map(|t| t.clone()).ok_or_else(|| Error(format!("{} not found in scope: `{}`", cat, id)))
}

pub fn eval_type(pat: &Pat, ctx: &TypeContext) -> Ret<Type> {
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
			let ty = eval_type(&pat, ctx);
			(0..n).map(|_| ty.clone()).collect::<Ret<_>>().map(Type::Tuple)
		},
	}
}

pub fn infer_type(exp: &Exp, ctx: &TypeContext) -> Ret<Type> {
	Ok(match exp {
		&Exp::Index(_) => Type::Any,
		&Exp::String(_) => Type::Any,
		&Exp::Var(ref id) => ctx.find_var_type(id)?,
		&Exp::Scope(ref decls, ref ret) => {
			let mut child = ctx.create_child();
			for decl in decls {
				apply_decl_type(decl, &mut child)?;
			}
			infer_type(ret, &child)?
		},
		&Exp::Expand(ref arg) => infer_type(arg, ctx)?,
		&Exp::Tuple(ref args) => Type::Tuple(args.iter().map(|e| infer_type(e, ctx)).collect::<Ret<_>>()?),
		&Exp::Concat(ref args) => Type::Concat(args.iter().map(|e| infer_type(e, ctx)).collect::<Ret<_>>()?),
		&Exp::Cond(_, ref then_exp, ref else_exp) => either_type(infer_type(then_exp, ctx)?, infer_type(else_exp, ctx)?),
		&Exp::Lambda(ref pat, ref body) => {
			// TODO type inference logic instead of special cases
			let ty = match (pat, &**body) {
				(&Pat::Var(ref id), &Exp::Extract(ref rc, ref cases)) if Exp::Var(id.clone()) == **rc =>
					infer_extract_arg_type(cases, ctx)?,
				_ => infer_pat_type(pat, ctx)?,
			};
			let mut fn_ctx = ctx.create_child();
			assign_pat_type(pat, &ty, &mut fn_ctx)?;
			Type::Func(Rc::new(ty), Rc::new(infer_type(body, &fn_ctx)?))
		},
		&Exp::Invoke(ref target, ref _arg) => {
			// TODO account for arg type
			match infer_type(target, ctx)? {
				Type::Func(_, ret) => (*ret).clone(),
				_ => Type::Any,
			}
		},
		&Exp::Repeat(n, ref exp) => {
			let ty = infer_type(exp, ctx)?;
			Type::Tuple((0..n).map(|_| ty.clone()).collect())
		},
		&Exp::State(ref arg) => infer_type(arg, ctx)?,
		&Exp::Phase(_, ref arg) => infer_type(arg, ctx)?,
		&Exp::Extract(ref _arg, ref cases) => {
			cases.iter()
				.map(|c| match c {
					&Case::Exp(_, ref e) => e,
					&Case::Default(ref e) => e,
				})
				.map(|e| infer_type(e, ctx))
				.fold(Ok(None), |a: Ret<Option<Type>>, b| Ok(Some(if let Some(a) = a? {either_type(a, b?)} else {b?})))?
				.unwrap_or(Type::Any)
		},
		&Exp::Anno(_, ref anno) => eval_type(anno, ctx)?,
	})
}

pub fn infer_extract_arg_type(cases: &Vec<Case>, ctx: &TypeContext) -> Ret<Type> {
	Ok(cases.iter()
		.flat_map(|c| match c {
			&Case::Exp(ref e, _) => Some(e).into_iter(),
			&Case::Default(_) => None.into_iter(),
		})
		.map(|e| infer_type(e, ctx))
		.fold(Ok(None), |a: Ret<Option<Type>>, b| Ok(Some(if let Some(a) = a? {either_type(a, b?)} else {b?})))?
		.unwrap_or(Type::Any))
}

pub fn infer_pat_type(pat: &Pat, ctx: &TypeContext) -> Ret<Type> {
	match pat {
		&Pat::Any => Ok(Type::Any),
		&Pat::Var(_) => Ok(Type::Any),
		&Pat::Tuple(ref args) => args.iter()
			.map(|p| infer_pat_type(p, ctx))
			.collect::<Ret<_>>()
			.map(Type::Tuple),
		&Pat::Concat(ref args) => args.iter()
			.map(|p| infer_pat_type(p, ctx))
			.collect::<Ret<_>>()
			.map(Type::Concat),
		&Pat::Anno(_, ref pat) => Ok(eval_type(pat, ctx)?),
		&Pat::Repeat(n, ref pat) => {
			let ty = infer_pat_type(&pat, ctx)?;
			Ok(Type::Tuple((0..n).map(|_| ty.clone()).collect()))
		},
	}
}

pub fn apply_decl_type(decl: &Decl, ctx: &mut TypeContext) -> Ret {
	match decl {
		&Decl::Let(ref pat, ref exp) => assign_pat_type(pat, &infer_type(exp, ctx)?, ctx),
		&Decl::Type(ref id, ref pat) => {
			let ty = eval_type(pat, ctx)?;
			ctx.add_type(id.clone(), ty)
		},
		&Decl::Data(ref id, ref vals) => ctx.add_datatype_type(id.clone(), vals.clone()),
		_ => Ok(()),
	}
}

pub fn assign_pat_type(pat: &Pat, ty: &Type, ctx: &mut TypeContext) -> Ret {
	match (pat, ty) {
		(&Pat::Any, _) => Ok(()),
		(&Pat::Var(ref id), _) => ctx.add_var_type(id.clone(), ty.clone()),
		(&Pat::Tuple(ref pats), &Type::Tuple(ref types)) => {
			if pats.len() != types.len() {err!("Cannot deconstruct {} types from {}", pats.len(), ty)}
			else {
				pats.iter().zip(types)
					.map(|(pat, t)| assign_pat_type(pat, t, ctx))
					.collect::<Ret<_>>()
			}
		},
		(&Pat::Anno(ref pat, ref anno), _) => assign_pat_type(pat, &eval_type(anno, ctx)?, ctx),
		_ => err!("{:?} cannot deconstruct type `{}`", pat, ty),
	}
}

pub fn either_type(a: Type, b: Type) -> Type {
	if a == b {a}
	else {Type::Any}
}