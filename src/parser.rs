use ast::*;

use std::rc::Rc;
use std::fs::File;
use std::io::prelude::*;

use nom;

pub type Error = String;

// macro_rules! term {
// 	($id:tt) => {ws!(tag!($id))};
// }

named!(ident<String>, ws!(map!(
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
	cases: delimited!(
		ws!(tag!("{")),
		many0!(extract_case),
		ws!(tag!("}"))
	) >>
	(Exp::Extract(Rc::new(exp), cases))
));

named!(extract_case<ExtractCase>, do_parse!(
	selector: exp >>
	ws!(tag!("=>")) >>
	result: exp >>
	(ExtractCase(selector, result))
));

named!(exp<Exp>,
	alt!(extract_exp | var_exp | tuple_exp | block_exp)
);

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
	choices: separated_list!(ws!(tag!("|")), data_val) >>
	(Decl::Data(id, choices))
));

named!(data_val<Ident>, do_parse!(
	id: ident >>
	(id)
));

named!(decl<Decl>,
	alt!(let_decl | data_decl)
);

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
	alt!(var_pat | tuple_pat)
);

// named!(string<&str>,
// 	delimited!(
// 		tag!("\""),
// 		map_res!(
// 		  escaped!(take_while1!(nom::is_alphanumeric), '\\', one_of!("\"n\\")),
// 		  str::from_utf8
// 		),
// 		tag!("\"")
// 	)
// );

// named!(array<Vec<JsonValue>>,
//   ws!(delimited!(
// 	tag!("["),
// 	separated_list!(tag!(","), value),
// 	tag!("]")
//   ))
// );

// named!(key_value<(&str, JsonValue)>,
//   ws!(separated_pair!(string, tag!(":"), value))
// );

// named!(hash<HashMap<String, JsonValue>>,
//   ws!(map!(
// 	delimited!(tag!("{"), separated_list!(tag!(","), key_value), tag!("}")),
// 	|tuple_vec| {
// 	  let mut h: HashMap<String, JsonValue> = HashMap::new();
// 	  for (k, v) in tuple_vec {
// 		h.insert(String::from(k), v);
// 	  }
// 	  h
// 	}
//   ))
// );

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