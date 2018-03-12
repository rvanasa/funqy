use ast::*;

use std::rc::Rc;
use std::fs::File;
use std::io::prelude::*;

use nom;

pub type Error = String;

// macro_rules! term {
// 	($id:tt) => {ws!(tag!($id))};
// }

named!(num_literal<usize>, ws!(map_res!(
	map_res!(take_while1!(nom::is_digit), ::std::str::from_utf8),
	|s: &str| s.parse()
)));

named!(name_ident<String>, ws!(map!(
	map_res!(take_while1!(nom::is_alphanumeric), ::std::str::from_utf8),
	|s| s.to_string()
)));

named!(opr_ident<String>, ws!(map!(
	map_res!(take_while1!(|c| "~!@#%^&*/?-+<>".contains(c as char)), ::std::str::from_utf8),
	|s| s.to_string()
)));

named!(ident<String>,
	alt!(name_ident | delimited!(ws!(tag!("(")), opr_ident, ws!(tag!(")"))))
);

named!(literal_exp<Exp>, map!(
	num_literal,
	Exp::Literal
));

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
	|vec| if vec.len() == 1 {vec[0].clone()} else {Exp::Tuple(vec)}
));

named!(block_exp<Exp>,
	delimited!(
		ws!(tag!("{")),
		scope_exp,
		ws!(tag!("}"))
	)
);

named!(scope_exp<Exp>, do_parse!(
	decls: many0!(terminated!(decl, opt!(ws!(tag!(";"))))) >>
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
	|vec| vec.into_iter().flat_map(|c| c).collect()
));

named!(default_case<Vec<Case>>, do_parse!(
	ws!(tag!("_")) >>
	result: case_result >>
	(vec![Case::Default(result)])
));

named!(exp_case<Vec<Case>>, do_parse!(
	selectors: separated_list!(ws!(tag!("|")), exp) >>
	result: case_result >>
	(selectors.into_iter().map(|selector| Case::Exp(selector, result.clone())).collect())
));

named!(case_result<Exp>, do_parse!(
	ws!(tag!("=>")) >>
	result: exp >>
	opt!(ws!(tag!(","))) >>
	(result)
));

named!(case<Vec<Case>>,
	alt!(default_case | exp_case)
);

named!(lambda_exp<Exp>, do_parse!(
	ws!(tag!("|")) >>
	pat: pat >>
	ws!(tag!("|")) >>
	exp: exp >>
	(Exp::Lambda(pat, Rc::new(exp)))
));

named!(path_exp<Exp>,
	alt!(extract_exp | literal_exp | var_exp | tuple_exp | block_exp | lambda_exp)
);

named!(decorated_exp<Exp>, do_parse!(
	path: path_exp >>
	invokes: many0!(tuple_exp) >>
	(invokes.into_iter().fold(path, |a, b| Exp::Invoke(Rc::new(a), Rc::new(b))))
));

named!(prefix_opr_exp<Exp>, do_parse!(
	opr: opr_ident >>
	exp: target_exp >>
	(Exp::Invoke(Rc::new(Exp::Var(opr)), Rc::new(exp)))
));

named!(target_exp<Exp>,
	alt!(prefix_opr_exp | decorated_exp)
);

named!(exp<Exp>, do_parse!(
	exp: target_exp >>
	infixes: many0!(tuple!(opr_ident, target_exp)) >>
	(infixes.into_iter().fold(exp, |a, (opr, b)| Exp::Invoke(
		Rc::new(Exp::Var(opr)),
		Rc::new(Exp::Tuple(vec![a, b])),
	)))
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

named!(data_val<Ident>, do_parse!(
	id: ident >>
	(id)
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
	opt!(ws!(tag!("="))) >>
	cases: extract_cases >>
	(Exp::Lambda(Pat::Var("$arg".to_string()), Rc::new(Exp::Extract(Rc::new(Exp::Var("$arg".to_string())), cases))))
));

named!(func_part<Exp>,
	alt!(func_basic_part | func_extract_part)
);

named!(assert_decl<Decl>, do_parse!(
	ws!(tag!("assert")) >>
	expect: exp >>
	ws!(tag!(":")) >>
	result: exp >>
	(Decl::Assert(expect, result))
));

named!(print_decl<Decl>, do_parse!(
	ws!(tag!("print")) >>
	exp: exp >>
	(Decl::Print(exp))
));

named!(decl<Decl>,
	alt!(let_decl | data_decl | func_decl | assert_decl | print_decl)
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
	|vec| if vec.len() == 1 {vec[0].clone()} else {Pat::Tuple(vec)}
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