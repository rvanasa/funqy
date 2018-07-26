#![feature(fs_read_write)]

extern crate rand;
extern crate regex;
extern crate reqwest;
extern crate num;
#[macro_use]
extern crate nom;
extern crate lapacke;
extern crate openblas_src;

#[macro_use]
pub mod error;
pub mod resource;
pub mod ast;
pub mod types;
pub mod engine;
pub mod eval;
pub mod parser;
pub mod stdlib;