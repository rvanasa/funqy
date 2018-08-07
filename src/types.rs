use error::*;
use eval::RunVal;
use ast::*;

use std::fmt;
use std::rc::Rc;

#[derive(Clone,Debug,PartialEq)]
pub struct DataType {
	pub id: Ident,
	pub variants: Vec<Ident>,
}

#[derive(Clone,Debug,PartialEq)]
pub enum Type {
	Any,
	Data(Rc<DataType>),
	Tuple(Vec<Type>),
	Concat(Vec<Type>),
}

impl fmt::Display for Type {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&Type::Any => write!(f, "_"),
			&Type::Data(ref rc) => write!(f, "{}", (*rc).id),
			&Type::Tuple(ref args) => write!(f, "({})", args.iter().map(|val| format!("{}", val)).collect::<Vec<_>>().join(", ")),
			&Type::Concat(ref args) => write!(f, "[{}]", args.iter().map(|val| format!("{}", val)).collect::<Vec<_>>().join(", ")),
		}
	}
}

impl Type {
	pub fn describes(&self, val: &RunVal) -> bool {
		self.assign(val.clone()).is_ok()
	}
	
	pub fn assign(&self, val: RunVal) -> Ret<RunVal> {
		match (self, val) {
			(Type::Any, val) => Ok(val),
			(Type::Tuple(ref types), RunVal::Tuple(ref args)) => {
				if types.len() != args.len() {
					err!("{} is not of length {}", RunVal::Tuple(args.to_vec()), types.len())
				}
				else {
					// TODO remove clone()
					types.iter().zip(args).map(|(p, a)| p.assign(a.clone())).collect::<Ret<_>>().map(RunVal::Tuple)
				}
			},
			// TODO concat assignments?
			(_, RunVal::Index(n)) => self.from_index(n),
			(_, RunVal::Data(_, n)) => self.from_index(n),
			(_, RunVal::State(state, _)) => {
				if self.size().map(|s| s != state.len()).unwrap_or(false) {
					err!("A state of size {} is not of type {}", state.len(), self)
				}
				else {Ok(RunVal::State(state, self.clone()))}
			},
			(_, val) => err!("{} is not of type {}", val, self)
		}
	}
	
	pub fn size(&self) -> Option<usize> {
		match self {
			Type::Any => None,
			Type::Data(ref dt) => Some((*dt.clone()).variants.len()),
			Type::Tuple(ref types) => types.iter().map(Type::size).fold(Some(1), |a, b| a.and_then(|a| b.map(|b| a * b))),
			Type::Concat(ref types) => types.iter().map(Type::size).fold(Some(1), |a, b| a.and_then(|a| b.map(|b| a + b))),
		}
	}
	
	pub fn from_index(&self, n: usize) -> Ret<RunVal> {
		match self {
			Type::Any => Ok(RunVal::Index(n)),
			Type::Data(ref dt) => Ok(RunVal::Data(dt.clone(), n)),
			Type::Tuple(ref types) => {
				let mut total_size = self.size().unwrap_or(0);
				let mut vals = vec![];
				for t in types {
					let size = t.size().ok_or_else(|| Error(format!("{} does not have a known size", t)))?;
					total_size /= size;
					vals.push(t.from_index((n / total_size) % size)?);
				}
				Ok(RunVal::Tuple(vals))
			},
			Type::Concat(_) => {
				err!("No index structure {} for type {}", n, self)
			},
		}
	}
}
