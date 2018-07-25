use error::*;
use resource;
use ast::*;
use engine::Phase;

use std::rc::Rc;
use regex::Regex;

use nom;

fn is_ident_char(c: u8) -> bool {
	nom::is_alphanumeric(c) || c == b'_'
}

fn is_opr_char(c: u8) -> bool {
	b"~!@#$%^&*/?|-+<>.".contains(&c)
}

fn with_anno(exp: Exp, anno: Option<Pat>) -> Exp {
	if let Some(pat) = anno {Exp::Anno(Rc::new(exp), pat)} else {exp}
}

named!(dec_literal<usize>, ws!(map_res!(
	map_res!(take_while1!(nom::is_digit), ::std::str::from_utf8),
	|s: &str| s.parse()
)));

named!(hex_literal<usize>, ws!(map_res!(
	map_res!(preceded!(tag!("0x"), take_while1!(nom::is_hex_digit)), ::std::str::from_utf8),
	|s: &str| usize::from_str_radix(s, 16)
)));

named!(bin_literal<usize>, ws!(map_res!(
	map_res!(preceded!(tag!("0b"), is_a!("01")), ::std::str::from_utf8),
	|s: &str| usize::from_str_radix(s, 2)
)));

named!(int_literal<isize>, do_parse!(
	sig: opt!(value!(-1, tag!("-"))) >>
	nat: index_literal >>
	(nat as isize * sig.unwrap_or(1))
));

named!(string_literal<String>, delimited!(
	tag!("\""),
	fold_many0!(
		alt!(
			pair!(tag!(r"\"), take!(1)) => {|(_, b)| b} |
			is_not!("\"")
		),
		String::new(),
		|a, b| format!("{}{}", a, String::from_utf8_lossy(b))
	),
	tag!("\"")
));

named!(index_literal<usize>,
	alt!(hex_literal | bin_literal | dec_literal)
);

named!(literal_exp<Exp>, alt!(
	index_literal => {Exp::Index} |
	string_literal => {Exp::String}
));

named!(name_ident<String>, ws!(map!(
	map_res!(take_while1!(is_ident_char), ::std::str::from_utf8),
	|s| s.to_string()
)));

named!(opr_ident<String>, ws!(map!(
	map_res!(take_while1!(is_opr_char), ::std::str::from_utf8),
	|s| s.to_string()
)));

named!(ident<String>,
	alt!(name_ident | delimited!(ws!(tag!("(")), opr_ident, ws!(tag!(")"))))
);

named!(var_exp<Exp>, map!(
	ident,
	Exp::Var
));

named!(tuple_exp<Exp>, map!(
	delimited!(
		ws!(tag!("(")),
		separated_list!(ws!(tag!(",")), arg_exp),
		ws!(tag!(")"))
	),
	|vec| if vec.len() == 1 && match vec[..] {[Exp::Tuple(_)] => false, _ => true} {vec[0].clone()} else {Exp::Tuple(vec)}
));

named!(concat_exp<Exp>, map!(
	delimited!(
		ws!(tag!("[")),
		separated_list!(ws!(tag!(",")), arg_exp),
		ws!(tag!("]"))
	),
	Exp::Concat
));

named!(block_exp<Exp>,
	delimited!(
		ws!(tag!("{")),
		scope_exp,
		ws!(tag!("}"))
	)
);

named!(scope_exp<Exp>, do_parse!(
	decls: many0!(terminated!(decl, opt!(complete!(ws!(tag!(";")))))) >>
	exp: opt!(complete!(exp)) >>
	(Exp::Scope(decls, Rc::new(exp.unwrap_or_else(|| Exp::Tuple(vec![])))))
));

named!(extract_exp<Exp>, do_parse!(
	ws!(tag!("extract")) >>
	exp: opt!(exp) >>
	cases: extract_cases >>
	(match exp {
		None => Exp::Lambda(
			Pat::Var("$arg".to_string()),
			Rc::new(Exp::Extract(Rc::new(Exp::Var("$arg".to_string())), cases))),
		Some(exp) => Exp::Extract(Rc::new(exp), cases),
	})
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
	selectors: separated_list!(ws!(tag!("|")), target_exp /**/) >>
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

named!(cond_exp<Exp>, do_parse!(
	cond_exp: preceded!(ws!(tag!("if")), exp) >>
	then_exp: preceded!(ws!(tag!("then")), exp) >>
	else_exp: preceded!(ws!(tag!("else")), exp) >>
	(Exp::Cond(Rc::new(cond_exp), Rc::new(then_exp), Rc::new(else_exp)))
));

named!(lambda_exp<Exp>, do_parse!(
	pat: delimited!(
		ws!(tag!("\\")),
		pat,
		ws!(tag!("->"))
	) >>
	exp: exp >>
	(Exp::Lambda(pat, Rc::new(exp)))
));

named!(phase_exp<Exp>, do_parse!(
	phase: preceded!(ws!(tag!("@")), delimited!(
		ws!(tag!("[")),
		tuple!(phase, opt!(map!(preceded!(ws!(tag!(",")), phase), |p| p * Phase::i()))),
		ws!(tag!("]"))
	)) >>
	exp: exp >>
	(Exp::Phase(phase.0 + phase.1.unwrap_or(::num::Zero::zero()), Rc::new(exp)))
));

named!(phase<Phase>, do_parse!(
	num: int_literal >>
	size: alt!(
		preceded!(ws!(tag!("/")), map!(int_literal, |n| n as f32)) |
		value!(100_f32, ws!(tag!("%"))) |
		value!(180_f32, ws!(tag!("d"))) |
		value!(::std::f32::consts::PI, ws!(tag!("r"))) |
		value!(1_f32)
	) >>
	(Phase::new(num as f32 / size, 0_f32))
));

named!(path_exp<Exp>,
	alt!(extract_exp | literal_exp | var_exp | tuple_exp | concat_exp | block_exp)
);

named!(decorated_exp<Exp>, do_parse!(
	path: path_exp >>
	invokes: many0!(tuple_exp) >>
	(invokes.into_iter().fold(path, |a, b| Exp::Invoke(Rc::new(a), Rc::new(b))))
));

named!(anno_exp<Exp>, do_parse!(
	exp: decorated_exp >>
	anno: opt_anno >>
	(with_anno(exp, anno))
));

named!(arg_exp<Exp>, alt!(
	preceded!(ws!(tag!("...")), exp) => {|exp| Exp::Expand(Rc::new(exp))} |
	exp
));

named!(prefix_opr_exp<Exp>, do_parse!(
	opr: opr_ident >>
	exp: target_exp >>
	(Exp::Invoke(Rc::new(Exp::Var(opr)), Rc::new(exp)))
));

named!(repeat_exp<Exp>, delimited!(
	ws!(tag!("(")),
	do_parse!(
		n: index_literal >>
		exp: target_exp >>
		(Exp::Repeat(n, Rc::new(exp)))
	),
	ws!(tag!(")"))
));

named!(target_exp<Exp>,
	alt!(phase_exp | prefix_opr_exp | cond_exp | anno_exp | lambda_exp | repeat_exp)
);

named!(exp<Exp>, do_parse!(
	exp: target_exp >>
	infixes: many0!(pair!(opr_ident, target_exp)) >>
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

named!(type_decl<Decl>, do_parse!(
	ws!(tag!("type")) >>
	id: ident >>
	ws!(tag!("=")) >>
	pat: pat >>
	(Decl::Type(id, pat))
));

named!(data_decl<Decl>, do_parse!(
	ws!(tag!("data")) >>
	id: ident >>
	ws!(tag!("=")) >>
	opt!(complete!(ws!(tag!("|")))) >>
	variant: data_val >>
	variants: many0!(preceded!(ws!(tag!("|")), data_val)) >>
	(Decl::Data(id, {
		let mut vs = vec![variant];
		vs.extend(variants);
		vs
	}))
));

named!(data_val<Ident>,
	alt!(ident)
);

named!(fn_decl<Decl>, do_parse!(
	ws!(tag!("fn")) >>
	id: ident >>
	part: fn_part >>
	(Decl::Let(Pat::Var(id), part))
));

named!(fn_basic_part<Exp>, do_parse!(
	pat: many1!(tuple_pat) >>
	anno: opt_anno >>
	ws!(tag!("=")) >>
	body: exp >>
	(pat.into_iter().rev().fold(with_anno(body, anno), |e, p| Exp::Lambda(p, Rc::new(e))))
));

named!(fn_extract_part<Exp>, do_parse!(
	opt!(ws!(tag!("="))) >>
	cases: extract_cases >>
	(Exp::Lambda(Pat::Var("$arg".to_string()), Rc::new(Exp::Extract(Rc::new(Exp::Var("$arg".to_string())), cases))))
));

named!(fn_part<Exp>,
	alt!(fn_basic_part | fn_extract_part)
);

named!(assert_decl<Decl>, do_parse!(
	ws!(tag!("assert")) >>
	expect: exp >>
	ws!(tag!("==")) >>
	result: exp >>
	(Decl::Assert(expect, result))
));

named!(print_decl<Decl>, do_parse!(
	ws!(tag!("print")) >>
	exp: exp >>
	(Decl::Print(exp))
));

named!(do_decl<Decl>, do_parse!(
	ws!(tag!("do")) >>
	exp: exp >>
	(Decl::Do(exp))
));

named!(decl<Decl>,
	alt!(let_decl | fn_decl | data_decl | type_decl | assert_decl | print_decl | do_decl)
);

named!(wildcard_pat<Pat>, do_parse!(
	ws!(tag!("_")) >>
	(Pat::Any)
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

named!(concat_pat<Pat>, map!(
	delimited!(
		ws!(tag!("[")),
		separated_list!(ws!(tag!(",")), pat),
		ws!(tag!("]"))
	),
	Pat::Concat
));

named!(repeat_pat<Pat>, delimited!(
	ws!(tag!("(")),
	do_parse!(
		n: index_literal >>
		pat: pat >>
		(Pat::Repeat(n, Rc::new(pat)))
	),
	ws!(tag!(")"))
));

named!(pat<Pat>, do_parse!(
	pat: alt!(repeat_pat | var_pat | wildcard_pat | tuple_pat | concat_pat) >>
	anno: opt_anno >>
	(if let Some(anno) = anno {Pat::Anno(Rc::new(pat), Rc::new(anno))} else {pat})
));

named!(opt_anno<Option<Pat>>, opt!(complete!(preceded!(ws!(tag!(":")), pat))));

pub fn parse_resource(path: &str) -> Ret<Exp> {
	parse(resource::load(path)?)
}

pub fn parse(input: String) -> Ret<Exp> {
	let input = input + "\n";
	let input = Regex::new("//[^\n]*\n").unwrap().replace_all(&input[..], " ");
	match scope_exp(input.as_bytes()) {
		nom::IResult::Done(s, exp) => {
			if s.len() == 0 {Ok(exp)}
			else {Err(Error(format!("Trailing input: {}", String::from_utf8_lossy(s))))}
		},
		nom::IResult::Error(err) => Err(Error(format!("Parse error: {}", err.description()))),
		nom::IResult::Incomplete(nom::Needed::Unknown) => Err(Error(format!("Incomplete input"))),
		nom::IResult::Incomplete(nom::Needed::Size(n)) => Err(Error(format!("Incomplete input ({})", n - input.len()))),
	}
}