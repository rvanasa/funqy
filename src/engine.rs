use rand::thread_rng;
use rand::distributions::{Weighted, WeightedChoice, Sample};

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

pub trait DebugPrint {
	fn print(&self);
}

impl DebugPrint for State {
	fn print(&self) {
		println!("{}", StateView(self));
	}
}

pub trait Stateful
where Self: ::std::marker::Sized {
	fn pad(self, n: usize) -> Self;
	fn combine(self, s: Self) -> Self;
	fn sup(self, s: Self) -> Self;
	fn extract(self, vs: Vec<Self>) -> Self;
	fn phase_flip(self) -> Self;
	fn measure(&self) -> usize;
}

impl Stateful for State {
	fn pad(mut self, n: usize) -> State {
		while self.len() < n {
			self.push(real!(0));
		}
		self
	}
	
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
		// zip(self, s, |x, y| (x + y) / real!(2).sqrt())
		create_sup(vec![self, s])
	}
	
	fn extract(self, vs: Vec<State>) -> State {
		// println!("{}  {:?}", StateView(&self),vs);////
		create_sup(self.into_iter().zip(vs).map(|(x, s)| {
			s.iter().map(|y| x * y).collect()
		}).collect())
	}
	
	fn phase_flip(self) -> State {
		self.into_iter().map(|x| -x).collect()
	}
	
	fn measure(&self) -> usize {
		let mut weights = vec![];
		for (i, t) in self.iter().enumerate() {
			weights.push(Weighted {
				weight: (real!(::std::u16::MAX) / (t/*abs*/ * t)).re as u32,
				item: i,
			});
		}
		let mut wc = WeightedChoice::new(&mut weights);
		let mut rng = thread_rng();
		wc.sample(&mut rng)
		// TODO persist measured value (fixed by universal no-cloning)
	}
}

// Create a superposition of the given states
pub fn create_sup(states: Vec<State>) -> State {
	let div = states.iter().map(|v| v.iter().fold(real!(0), |a, b| a + b * b))
		.fold(real!(0), |a, b| a + b).sqrt();
	
	states.into_iter().fold(vec![], |a, b| zip(a, b, |x, y| x + y))
		.into_iter().map(|x| x / div).collect()
}

// fn normalize(state: State) -> State {
// 	let mag = state.iter().fold(real!(0), |a, b| a + (b * b));
// 	state.into_iter().map(|x| x / mag.sqrt()).collect()
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
	let max_len = ::std::cmp::max(a.len(), b.len());
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

use std::fmt;
pub struct StateView<'a>(pub &'a State);
impl<'a> fmt::Display for StateView<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "[{}]", self.0.iter().map(|d| format!("{}", round(*d, 4))).collect::<Vec<String>>().join(", "))
	}
}

fn round(f: Cf32, d: i32) -> Cf32 {
	use num::complex::Complex;
	let m = Complex::new(10_f32.powi(d), 0_f32);
	let f = f * m;
	Complex::new(f.re.round(), f.im.round()) / m
}