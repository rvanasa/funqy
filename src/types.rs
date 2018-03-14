use ast::*;

use std::rc::Rc;

#[derive(Clone,Debug,PartialEq)]
pub struct DataType {
	pub variants: Vec<Ident>,
}

// Type
#[derive(Clone,Debug,PartialEq)]
pub enum Type {
	Data(Rc<DataType>),
	Tuple(Vec<Type>),
}