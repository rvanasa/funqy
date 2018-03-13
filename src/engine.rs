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
pub type Gate = Vec<State>;
pub type Phase = f32;

pub trait DebugPrint {
	fn print(&self);
}

impl DebugPrint for State {
	fn print(&self) {
		println!("{}", StateView(self));
	}
}

impl DebugPrint for Gate {
	fn print(&self) {
		println!("[");
		self.iter().for_each(|s| s.print());
		println!("]");
	}
}

pub trait Stateful
where Self: ::std::marker::Sized {
	fn pad(self, n: usize) -> Self;
	fn sup(self, s: Self) -> Self;
	fn extract(self, vs: Vec<Self>) -> Self;
	fn phase(self, p: Phase) -> Self;
	fn phase_flip(self) -> Self;
	fn measure(self) -> usize;
}

impl Stateful for State {
	fn pad(mut self, n: usize) -> State {
		while self.len() < n {
			self.push(real!(0));
		}
		self
	}
	
	fn sup(self, s: State) -> State {
		create_sup(vec![self, s])
	}
	
	fn extract(self, vs: Gate) -> State {
		create_sup(self.into_iter().zip(vs).map(|(x, s)| {
			s.iter().map(|y| x * y).collect()
		}).collect())
	}
	
	fn phase(self, p: Phase) -> State {
		let (cos, sin) = (p.cos(), p.sin());
		self.into_iter().map(|x| Complex::new(
			cos * x.re + sin * x.im,
			sin * x.re + cos * x.im)).collect()
	}
	
	fn phase_flip(self) -> State {
		self.into_iter().map(|x| -x).collect()
	}
	
	fn measure(self) -> usize {
		let mut weights = vec![];
		for (i, n) in self.into_iter().enumerate() {
			weights.push(Weighted {
				weight: (::std::u16::MAX as f32 / absq(n)) as u32,
				item: i,
			});
		}
		let mut wc = WeightedChoice::new(&mut weights);
		let mut rng = thread_rng();
		wc.sample(&mut rng)
	}
}

pub trait Combine {
	fn combine(self, Self) -> Self;
}

impl Combine for State {
	fn combine(self, s: State) -> State {
		let mut state = vec![];
		for x in self {
			for y in s.iter() {
				state.push(x * y);
			}
		}
		state
	}
}

impl Combine for Gate {
	fn combine(self, g: Gate) -> Gate {
		let mut dims = vec![];
		for x in self {
			for y in g.iter() {
				dims.push(x.clone().combine(y.clone()));
			}
		}
		dims
	}
}

pub trait Transpose {
	fn transpose(self) -> Self;
}

impl Transpose for Gate {
	fn transpose(self) -> Gate {
		let max_len = self.iter().map(|s| s.len()).max().unwrap_or_else(|| 0);
		let mut dims: Gate = vec![];
		for i in 0..max_len {
			let mut dim: State = vec![];
			for s in self.iter() {
				dim.push(match s.get(i) {
					Some(&n) => n,
					None => real!(0),
				});
			}
			dims.push(dim);
		}
		dims
	}
}

// Create a superposition of the given states
pub fn create_sup(states: Vec<State>) -> State {
	let div = states.iter().map(|v| v.iter().fold(0_f32, |a, b| a + absq(*b)))
		.fold(0_f32, |a, b| a + b).sqrt();
	
	states.into_iter().fold(vec![], |a, b| zip(a, b, |x, y| x + y))
		.into_iter().map(|x| x / div).collect()
}

// fn normalize(state: State) -> State {
// 	let mag = state.iter().fold(real!(0), |a, b| a + (b * b));
// 	state.into_iter().map(|x| x / mag.sqrt()).collect()
// }

// Create a unit vector state in the given Hilbert dimension
pub fn get_state(n: usize) -> State {
	if n < 0 /* || n >= max_state_size */ {
		panic!("Invalid state size: {}", n);
	}
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

fn absq(n: Cf32) -> f32 {
	n.re * n.re + n.im * n.im
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