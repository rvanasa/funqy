use engine::Phase;

use std::rc::Rc;

pub type Ident = String;

// Pattern (e.g. function parameters, match/extract cases)
#[derive(Clone,Debug,PartialEq)]
pub enum Pat {
	Any,
	Var(Ident),
	Tuple(Vec<Pat>),
	// Data(Ident, PatRc),
	Anno(Rc<Pat>, Rc<Pat>),
}

// Scope declaration (statement)
#[derive(Clone,Debug,PartialEq)]
pub enum Decl {
	Let(Pat, Exp),
	Type(Ident, Pat),
	Data(Ident, Vec<Ident>),
	Assert(Exp, Exp),
	Print(Exp),
	// Func(Pat, Exp),
}

// Expression
type ExpRc = Rc<Exp>;
#[derive(Clone,Debug,PartialEq)]
pub enum Exp {
	Index(usize),
	String(String),
	Var(Ident),
	Scope(Vec<Decl>, ExpRc),
	Expand(ExpRc),
	Tuple(Vec<Exp>),
	Concat(Vec<Exp>),
	Cond(ExpRc, ExpRc, ExpRc),
	Lambda(Pat, ExpRc),
	Invoke(ExpRc, ExpRc),
	State(ExpRc),
	Phase(Phase, ExpRc),
	Extract(ExpRc, Vec<Case>),
	Anno(ExpRc, Pat),
}

#[derive(Clone,Debug,PartialEq)]
pub enum Case {
	Exp(Exp, Exp),
	Default(Exp),
}
