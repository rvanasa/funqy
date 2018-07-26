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
pub type Phase = Cf32;

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
	fn normalized(self) -> Self;
	fn phase(self, p: Phase) -> Self;
	fn phase_flip(self) -> Self;
	fn prob_sum(&self) -> f32;
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
	
	fn normalized(self) -> State {
		let div = self.prob_sum().sqrt();
		self.into_iter().map(|s| s / div).collect()
	}
	
	fn phase(self, p: Phase) -> State {
		let n = p * ::std::f32::consts::PI;
		let (cos, sin) = (n.cos(), n.sin());
		self.into_iter().map(|x| Complex::new(//TODO implement imaginary phases
			cos.re * x.re + sin.re * x.im,
			sin.re * x.re + cos.re * x.im,
		)).collect()
	}
	
	fn phase_flip(self) -> State {
		self.into_iter().map(|x| -x).collect()
	}
	
	fn prob_sum(&self) -> f32 {
		self.iter().fold(0_f32, |a, &b| a + absq(b))
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

pub trait Extract {
	fn extract(self, Gate) -> Self;
}

impl Extract for State {
	fn extract(self, g: Gate) -> State {
		create_sup(self.into_iter().zip(g).map(|(x, s)| {
			s.iter().map(|y| x * y).collect()
		}).collect())
	}
}

impl Extract for Gate {
	fn extract(self, g: Gate) -> Gate {
		self.into_iter().map(|state| state.extract(g.clone())).collect() /////??
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

pub trait MatrixLike {
	fn width(&self) -> usize;
	
	fn is_unitary(&self) -> bool;
	
	fn inverse(self) -> Self;
	fn negate(self) -> Self;
	fn power(self, Phase) -> Self;
}

impl MatrixLike for Gate {
	fn width(&self) -> usize {self.iter().map(|s| s.len()).max().unwrap_or_else(|| 0)}
	
	fn is_unitary(&self) -> bool {
		unimplemented!()
		// // self * self.inv() == identity
		// for (i, s) in self.iter().enumerate() {
		// 	for (j, n) in s[(i + 1)..].iter().enumerate() {
		// 		...
		// 	}
		// }
		// true
	}
	
	fn inverse(self) -> Gate {
		// Requires unitary matrix
		let mut dims: Gate = vec![];
		for i in 0..self.width() {
			let mut dim: State = vec![];
			for s in self.iter() {
				dim.push(match s.get(i) {
					Some(&n) => n.conj(),
					None => real!(0),
				});
			}
			dims.push(dim);
		}
		dims
	}
	
	fn negate(self) -> Self {
		self.into_iter().map(|dim| dim.into_iter().map(|s| -s).collect()).collect()
	}
	
	fn power(self, p: Phase) -> Self {
		use lapacke::*;
		use num::Zero;
		use num::One;
		if p.is_one() {
			return self
		}
		
		let size = ::std::cmp::max(self.len(), self.width());
		let mut mat = vec![c64::zero(); size * size];
		for (i, s) in self.into_iter().enumerate() {
			for (j, n) in s.into_iter().enumerate() {
				mat[i * size + j] = c64::new(n.re as f64, n.im as f64);
			}
		}
		
		let mut vals = vec![c64::zero(); size];
		let mut vecs = vec![c64::zero(); size * size];
		// let mut ivecs: Vec<c64>;
		unsafe {
			fn wrap_status(status: i32) {
				if status != 0 {
					panic!(status);
				}
			}
			// let mut pivots = vec![0; size];
			let size = size as i32;
			wrap_status(zgeev(Layout::RowMajor, b'N', b'V', size, &mut mat[..], size, &mut vals[..], &mut [], size, &mut vecs[..], size));
			// ivecs = vecs.clone();
			// wrap_status(zgetrf(Layout::RowMajor, size, size, &mut ivecs, size, &mut pivots));
			// wrap_status(zgetri(Layout::RowMajor, size, &mut ivecs, size, &pivots[..]));
		}
		
		fn to_state(v: Vec<c64>) -> State {
			v.into_iter().map(|c| Cf32::new(c.re as f32, c.im as f32)).collect()
		}
		fn to_gate(v: Vec<c64>, n: usize) -> Gate {
			(0..n).map(|i| to_state(v[n * i .. n * (i + 1)].to_vec())).collect()
		}
		let vals = to_state(vals);
		let vecs = to_gate(vecs, size);
		// let ivecs = to_gate(ivecs, size);
		let ivecs = vecs.clone().inverse();
		
		let diag = (0..size).map(|i| {
			let mut vec = vec![real!(0); size];
			vec[i] = vals[i] * p;
			vec
		}).collect();
		ivecs.extract(diag).extract(vecs)
	}
}

// Create a superposition of the given states
pub fn create_sup(states: Vec<State>) -> State {
	let div = states.iter().map(|v| v.prob_sum())
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
	let mut state = vec![];
	for _ in 0..n {
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
	let m = Complex::new(10_f32.powi(d), 0_f32);
	let f = f * m;
	Complex::new(f.re.round(), f.im.round()) / m
}