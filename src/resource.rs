use error::*;

use std::fs;
use reqwest;

pub fn with_ext<'a>(path: &str, ext: &'a str) -> String {
	let ext = format!(".{}", ext);
	if !path.ends_with(ext.as_str()) {
		format!("{}{}", path, ext)
	}
	else {path.to_string()}
}

pub fn load(path: &str) -> Ret<String> {
	if path.starts_with("http://") || path.starts_with("https://") {
		Ok(reqwest::get(path)?.text()?)
	}
	else {String::from_utf8(fs::read(path)?).map_err(|err| Error(format!("{}", err)))}
}
