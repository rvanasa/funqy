use std::rc::Rc;

pub type Ident = String;

// Pattern (e.g. function parameters, match/extract cases)
type PatRc = Rc<Pat>;
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum Pat {
	Unit,
	Var(Ident),
	Tuple(Vec<Pat>),
	// Data(Ident, PatRc),
}

// Scope declaration (statement)
type DeclRc = Rc<Decl>;
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum Decl {
	Data(Ident, Vec<Ident>),
	// Type(Ident, Type),
	Let(Pat, Exp),
	// Func(Pat, Exp),
}

// Expression
type ExpRc = Rc<Exp>;
#[derive(Clone,Debug,Eq,PartialEq)]
pub enum Exp {
	Var(Ident),
	Scope(Vec<Decl>, ExpRc),
	Tuple(Vec<Exp>),
	// Data(Ident),
	State(ExpRc),
	Extract(ExpRc, Vec<ExtractCase>),
	Sup(ExpRc, ExpRc),
	Measure(ExpRc),
}

// Extract dimension case
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct ExtractCase(pub Exp, pub Exp);
