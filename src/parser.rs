use ast::*;

use std::rc::Rc;
use std::fs::File;
use std::io::prelude::*;

use nom;

pub type Error = String;

// macro_rules! term {
// 	($id:tt) => {ws!(tag!($id))};
// }

named!(ident<String>, ws!(map!( // TODO ensure first char is non-numeric
	map_res!(take_while1!(nom::is_alphanumeric), ::std::str::from_utf8),
	|s| s.to_string()
)));

named!(var_exp<Exp>, map!(
	ident,
	Exp::Var
));

named!(tuple_exp<Exp>, map!(
	delimited!(
		ws!(tag!("(")),
		separated_list!(ws!(tag!(",")), exp),
		ws!(tag!(")"))
	),
	Exp::Tuple
));

named!(block_exp<Exp>,
	delimited!(
		ws!(tag!("{")),
		scope_exp,
		ws!(tag!("}"))
	)
);

named!(scope_exp<Exp>, do_parse!(
	decls: many0!(decl) >>
	exp: exp >>
	(Exp::Scope(decls, Rc::new(exp)))
));

named!(extract_exp<Exp>, do_parse!(
	ws!(tag!("extract")) >>
	exp: exp >>
	cases: extract_cases >>
	(Exp::Extract(Rc::new(exp), cases))
));

named!(extract_cases<Vec<Case>>, map!(
	delimited!(
		ws!(tag!("{")),
		many0!(case),
		ws!(tag!("}"))
	),
	|vec| vec.into_iter().flat_map(move |c| c).collect()
));

named!(default_case<Vec<Case>>, do_parse!(
	ws!(tag!("_")) >>
	ws!(tag!("=>")) >>
	result: exp >>
	opt!(ws!(tag!(","))) >>
	(vec![Case::Default(result)])
));

named!(exp_case<Vec<Case>>, do_parse!(
	selectors: separated_list!(ws!(tag!("|")), exp) >>
	ws!(tag!("=>")) >>
	result: exp >>
	opt!(ws!(tag!(","))) >>
	(selectors.into_iter().map(|selector| Case::Exp(selector, result.clone())).collect())
));

named!(case<Vec<Case>>,
	alt!(default_case | exp_case)
);

named!(path_exp<Exp>,
	alt!(extract_exp | var_exp | tuple_exp | block_exp)
);

named!(exp<Exp>, do_parse!(
	path: path_exp >>
	invokes: many0!(tuple_exp) >>
	(invokes.into_iter().fold(path, move |a, b| Exp::Invoke(Rc::new(a), Rc::new(b))))
));

named!(let_decl<Decl>, do_parse!(
	ws!(tag!("let")) >>
	pat: pat >>
	ws!(tag!("=")) >>
	exp: exp >>
	(Decl::Let(pat, exp))
));

named!(data_decl<Decl>, do_parse!(
	ws!(tag!("data")) >>
	id: ident >>
	ws!(tag!("=")) >>
	variants: separated_list!(ws!(tag!("|")), data_val) >>
	(Decl::Data(id, variants))
));

named!(func_decl<Decl>, do_parse!(
	ws!(tag!("fn")) >>
	id: ident >>
	part: func_part >>
	(Decl::Let(Pat::Var(id), part))
));

named!(func_basic_part<Exp>, do_parse!(
	pat: tuple_pat >>
	ws!(tag!("=")) >>
	body: exp >>
	(Exp::Lambda(pat, Rc::new(body)))
));

named!(func_extract_part<Exp>, do_parse!(
	ws!(tag!("=")) >>
	cases: extract_cases >>
	(Exp::Lambda(Pat::Var("@arg".to_string()), Rc::new(Exp::Extract(Rc::new(Exp::Var("@arg".to_string())), cases))))
));

named!(func_part<Exp>,
	alt!(func_basic_part | func_extract_part)
);

named!(data_val<Ident>, do_parse!(
	id: ident >>
	(id)
));

named!(decl<Decl>,
	alt!(let_decl | data_decl | func_decl)
);

named!(wildcard_pat<Pat>, do_parse!(
	ws!(tag!("_")) >>
	(Pat::Wildcard)
));

named!(var_pat<Pat>, map!(
	ident,
	Pat::Var
));

named!(tuple_pat<Pat>, map!(
	delimited!(
		ws!(tag!("(")),
		separated_list!(ws!(tag!(",")), pat),
		ws!(tag!(")"))
	),
	Pat::Tuple
));

named!(pat<Pat>,
	alt!(wildcard_pat | var_pat | tuple_pat)
);

pub fn parse_file(path: &str) -> Result<Exp, Error> {
	let mut file = File::open(&path).expect("Could not open file"); // convert to Result
	let mut input = String::new();
	file.read_to_string(&mut input).expect("Could not read from file");
	parse(input)
}

pub fn parse(input: String) -> Result<Exp, Error> {
	match scope_exp(input.as_bytes()) {
		nom::IResult::Done(s, exp) => {
			if s.len() == 0 {Ok(exp)}
			else {Err(format!("Trailing input: {}", match ::std::str::from_utf8(s) {
				Ok(s) => s,
				Err(_) => "<?>",
			}))}
		},
		nom::IResult::Error(err) => Err(format!("Parse error: {}", err.description())),
		nom::IResult::Incomplete(nom::Needed::Unknown) => Err(format!("Incomplete input")),
		nom::IResult::Incomplete(nom::Needed::Size(n)) => Err(format!("Incomplete input ({})", n)),
	}
}