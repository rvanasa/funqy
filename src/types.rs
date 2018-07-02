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
}

impl Type {
	pub fn describes(&self, val: &RunVal) -> bool {
		self.assign(val.clone()).is_ok()
	}
	
	pub fn assign(&self, val: RunVal) -> Ret<RunVal> {
		match (self, val) {
			(Type::Any, val) => Ok(val),
			(Type::Data(ref dt), RunVal::Index(n)) => Ok(RunVal::Data(dt.clone(), n)),
			(Type::Tuple(ref params), RunVal::Tuple(ref args)) => {
				if params.len() != args.len() {
					Err(Error(format!("{} is not of length {}", RunVal::Tuple(args.to_vec()), params.len())))
				}
				else {
					// TODO remove clone()
					params.iter().zip(args).map(|(p, a)| p.assign(a.clone())).collect::<Ret<_>>().map(RunVal::Tuple)
				}
			}
			(t, val) => Err(Error(format!("{} is not of type {}", val, t))),
		}
	}
}

impl fmt::Display for Type {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&Type::Any => write!(f, "_"),
			&Type::Data(ref rc) => write!(f, "{}", (*rc).id),
			&Type::Tuple(ref args) => write!(f, "({})", args.iter().map(|val| format!("{}", val)).collect::<Vec<_>>().join(", ")),
		}
	}
}
