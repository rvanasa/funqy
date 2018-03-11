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
	fn pad(self, n: usize) -> State {
		let mut state = self;
		while state.len() < n {
			state.push(real!(0));
		}
		state
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
		self.into_iter().zip(vs).map(|(x, s)| {
			s.iter().map(|y| x * y).collect()
		}).fold(vec![], |t, s: State| {
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
		self.into_iter().map(|x| -x).collect()
	}
	
	fn measure(&self) -> usize {
		let mut weights = vec![];
		for (i, t) in self.iter().enumerate() {
			weights.push(Weighted {
				item: i,
				weight: (t/*.abs()*/ * t).re as u32,
			});
		}
		let mut wc = WeightedChoice::new(&mut weights);
		let mut rng = thread_rng();
		wc.sample(&mut rng)
	}
}

// Create a superposition of the given states
pub fn create_sup(states: Vec<State>) -> State {
	let len = states.len();
	states.into_iter().fold(vec![], |a, b| zip(a, b, |x, y| x + y))
		.into_iter().map(|x| x / real!(len).sqrt()).collect()
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
