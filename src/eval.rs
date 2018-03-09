use std;

use rand::*;

use num::complex::Complex;

pub type Cf32 = Complex<f32>;

#[macro_export]
macro_rules! real {
	($n:expr) => {Complex::new(($n) as f32, 0_f32)}
}

#[macro_export]
macro_rules! imag {
	($n:expr) => {Complex::new(0_f32, ($n) as f32)}
}

pub type State = Vec<Cf32>;

pub trait Stateful
where Self: std::marker::Sized {
	fn combine(self, s: Self) -> Self;
	fn sup(self, s: Self) -> Self;
	fn extract(self, vs: Vec<Self>) -> Self;
	fn phase_flip(self) -> Self;
	fn measure(self) -> usize;
}

impl Stateful for State
where Self: std::marker::Sized {
	fn combine(self, s: State) -> State {
		let mut state = vec![];
		for x in self {
			for y in s.iter() {
				state.push(x * y);
			}
		}
		state
	}
	
	fn sup(self, s: State) -> State {
		zip(self, s, move |x, y| x + y)
	}
	
	fn extract(self, vs: Vec<State>) -> State {
		self.into_iter().zip(vs).map(move |(x, s)| {
			s.iter().map(move |y| x * y).collect()
		}).fold(vec![], move |t, s: State| {
			let mut t = t;
			while t.len() < s.len() {
				t.push(real!(0));
			}
			for i in 0..s.len() {
				t[i] += s[i];
			}
			t
		})
	}
	
	fn phase_flip(self) -> State {
		self.into_iter().map(move |x| -x).collect()
	}
	
	fn measure(self) -> usize {
		0//////
	}
}

// fn normalize(state: State) -> State {
// 	let mag = state.iter().fold(real!(0), move |a, b| a + (b * b));
// 	state.into_iter().map(move |x| x / mag.sqrt()).collect()
// }

// Create a unit vector state in the given Hilbert dimension
pub fn get_state(n: usize) -> State {
	let mut state = vec![];
	for i in 0..n {
		state.push(real!(0));
	}
	state.push(real!(1));
	state
}

fn zip<T>(a: State, b: State, f: T) -> State
where T: Fn(Cf32, Cf32) -> Cf32 {
	let zero = real!(0);
	// let (a, b) = if a.len() > b.len() {(a, b)} else {(b, a)};
	let max_len = std::cmp::max(a.len(), b.len());
	let mut a = a;
	let mut b = b;
	while a.len() < max_len {
		a.push(zero);
	}
	while b.len() < max_len {
		b.push(zero);
	}
	
	let mut state = vec![];
	for i in 0..max_len {
		state.push(f(a[i], b[i]));
	}
	state
}