use std::rc::Rc;

pub type Ident = String;

// Pattern (e.g. function parameters, match/extract cases)
#[derive(Clone,Debug,PartialEq)]
pub enum Pat {
	Wildcard,
	Var(Ident),
	Tuple(Vec<Pat>),
	// Data(Ident, PatRc),
}

// Scope declaration (statement)
#[derive(Clone,Debug,PartialEq)]
pub enum Decl {
	Data(Ident, Vec<Ident>),
	// Type(Ident, Type),
	Let(Pat, Exp),
	Assert(Exp, Exp),
	Print(Exp),
	// Func(Pat, Exp),
}

// Expression
type ExpRc = Rc<Exp>;
#[derive(Clone,Debug,PartialEq)]
pub enum Exp {
	Nat(usize),
	String(String),
	Var(Ident),
	Scope(Vec<Decl>, ExpRc),
	Tuple(Vec<Exp>),
	Lambda(Pat, ExpRc),
	Invoke(ExpRc, ExpRc),
	State(ExpRc),
	Phase(::engine::Phase, ExpRc),
	Extract(ExpRc, Vec<Case>),
}

#[derive(Clone,Debug,PartialEq)]
pub enum Case {
	Exp(Exp, Exp),
	Default(Exp),
}
