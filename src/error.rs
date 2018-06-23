use std::fmt;
use std::io;
use reqwest;
use nom;

pub type Ret<T = ()> = Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Error(pub String);

impl From<io::Error> for Error {
	fn from(error: io::Error) -> Self {
		Error(format!("{:?}", error.kind()))
	}
}

impl<I> From<nom::Err<I>> for Error where I: fmt::Debug {
	fn from(err: nom::Err<I>) -> Self {
		// macro_rules! display_context {
		// 	($id: expr) => {
		// 		match $id {
		// 			nom::Context::Code(_input, kind) => format!("{:?}", kind),
		// 		}
		// 	}
		// }
		Error(match err {
			// nom::Err::Error(ctx) => display_context!(ctx),
			// nom::Err::Failure(ctx) => display_context!(ctx),
			_ => format!("{:?}", err)
		})
	}
}

impl From<reqwest::Error> for Error {
	fn from(error: reqwest::Error) -> Self {
		Error(format!("{:?}", error))
	}
}
