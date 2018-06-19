#![feature(fs_read_write)]

extern crate regex;
extern crate rand;
extern crate num;
#[macro_use]
extern crate nom;
extern crate ndarray;
extern crate ndarray_linalg;
// extern crate openblas_src;

pub mod error;
pub mod ast;
pub mod types;
pub mod engine;
pub mod eval;
pub mod parser;
pub mod stdlib;