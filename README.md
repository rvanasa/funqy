## FunQy: A Next-Generation Quantum Programming Language

FunQy is a novel architecture-agnostic functional quantum programming language. 
Instead of regarding algorithms in terms of [qubits](https://en.wikipedia.org/wiki/Qubit) and [logic gates](https://en.wikipedia.org/wiki/Quantum_logic_gate), 
FunQy can simulate any combination of quantum objects using what we call _pattern extraction_.

Pattern extraction is a bidirectional analog to pattern matching from classical functional programming, 
with the additional quantum capability of executing all paths simultaneously. 
This abstraction provides a clear understanding of the logic and quantum performance benefits of a particular program—instead
of executing one path at a time, pattern extraction can execute arbitrary combinations of possible inputs. 
This tends to be vastly more intuitive and scalable than the prevalent [circuit-based algorithm](https://arxiv.org/abs/1804.03719) conventions. 

Most of the existing "high-level" quantum programming languages are just preprocessors for defining these quantum circuits.
FunQy exposes powerful layers of abstraction that are fully independent of the underlying architecture.

---

Here are a few interesting outcomes of this paradigm:
- Funqy looks and feels like a high-level programming language, **useful for classical software engineers** unfamiliar with quantum gates and registers.
- All states and values are immutable and thus **purely functional**. This declarative basis for quantum computation is more intuitive, more optimizable, and more powerful than the quantum circuit paradigm. 
- The language is fully **architecture-agnostic**; qubits and gates are completely invisible to the language unless otherwise desired.
- **Classical and quantum algorithms are defined simulaneously**; in other words, the compiler will use the classical version of a function if the input value is correspondingly classical. In effect, only operations which would actually benefit from quantum speed-up are performed on a quantum register.
- By organizing code in terms of functions and extractions, scripts tend to **semantically convey their underlying purpose and logic**. 
- On top of "multiplicative" space (entanglement/tuples), FunQy unlocks the **"additive" space (matrix/vector indices)** of a quantum system.
- It is possible to define **non-unitary mappings** (i.e. non-square and/or non-reversible matrices), which compile using auxillary qubits as needed.
- State initializations, repeated measurements, and dynamically adjusted circuits are all implicit to FunQy's semantics. For instance, reusing a state object will automatically reconstruct the state to circumvent the no-cloning principle. 
- FunQy's type system provides the **scalability and expressiveness** needed to design and reason about algorithms for future (100+ qubit) quantum computers. 
- `extract` blocks **visually demonstrate quantum algorithm speed-up** by always having the same time complexity regardless of input value. 

---

### Build Requirements

- [Nightly Rust](https://doc.rust-lang.org/1.15.1/book/nightly-rust.html) `>= 1.28.0`
- [gfortran](http://laptops.eng.uci.edu/software-installation/getting-started-with-programming/fortran-tutorial?tmpl=%2Fsystem%2Fapp%2Ftemplates%2Fprint%2F&showPrintDialog=1) (required for LAPACKE) `>= 4.8`

### Usage Examples

Evaluate a FunQy script:
```sh
$ funqy eval path/to/ScriptFile.fqy [-o output_file.txt] [--watch]
$ funqy eval github:rvanasa/funqy:tests/scripts/Test.fqy [...]
$ funqy eval "raw: measure(sup(1,2,3))" [...]
```

Start an interactive REPL session:
```sh
$ funqy repl [-h history_file.txt]
```

View all available commands:
```sh
$ funqy --help
```

### Qubit Gate Analogy

```

data Qubit = F | T		//	Define |0⟩ as `F` and |1⟩ as `T`

let (^) = sup			// Define superposition operator
let (~) = phf			// Define phase flip operator
let (#) = measure		// Define measurement operator

// identity (no change)
fn id = {
	F => F,				//	|0⟩ => |0⟩
	T => T,				//	|1⟩ => |1⟩
}

// Pauli-X rotation (NOT gate)
fn px = {
	F => T,				//	|0⟩ => |1⟩
	T => F,				//	|1⟩ => |0⟩
}
let not = px
let (!) = px

// Pauli-Y rotation
fn py = {
	F => @[1/2] T,		//	|0⟩ => |i⟩
	T => @[-1/2] F,		//	|1⟩ => -|i⟩
}

// Pauli-Z rotation
fn pz = {
	F => F,				//	|0⟩ => |0⟩
	T => ~T,			//	|1⟩ => -|1⟩
}

// Hadamard gate
fn hadamard = {
	F => F ^ T, 		//	|0⟩ => (|0⟩ + |1⟩) / sqrt(2)
	T => F ^ ~T,		//	|1⟩ => (|0⟩ - |1⟩) / sqrt(2)
}

// Alternate implementation using if/then/else statement
fn hadamard_cond(s) =
	if s then F ^ T else F ^ ~T

// SWAP gate
fn swap = {
	(F, T) => (T, F), 	//	|01⟩ => |10⟩
	(T, F) => (F, T),	//	|10⟩ => |01⟩
}

// sqrt(NOT) gate
let sqrt_not = @[1/2] not

// sqrt(SWAP) gate
let sqrt_swap = @[1/2] swap

// Controlled gate
fn c(gate)(ctrl, tgt) = {
	let out = extract ctrl {
		F => tgt,	//	|0⟩ ⊗ tgt => |0⟩ ⊗ tgt 
		T => gate(tgt),	//	|1⟩ ⊗ tgt => |0⟩ ⊗ gate(tgt)
	}
	(ctrl, out)
}

// Controlled NOT gate
fn cnot(ctrl, tgt) = c(not)(ctrl, tgt)

// Bell state preparation (implemented via gates)
fn bell_as_circuit(q1, q2) = cnot(hadamard(q1), q2)

// Bell state preparation (implemented via extraction)
fn bell_as_extract = {
	(F, F) => (F, F) ^ (T, T),
	(F, T) => (F, T) ^ (T, F),
	(T, F) => (F, F) ^ ~(T, T),
	(T, T) => (F, T) ^ ~(T, F),
}

assert bell_as_circuit == bell_as_extract

let inv_bell = inv(bell_as_circuit)
assert inv(inv_bell) == bell_as_circuit

```

_Note that FunQy is very early in development; this syntax may be subject to change._

The above example demonstrates the crossover between FunQy and traditional quantum computing languages. 
However, the pattern extraction paradigm gains its advantage from combining different quantum object dimensionalities. 

### Higher-Order Gate Analogy

Here is an interesting outcome of using both 2D (qubit) and 3D (qutrit) values in a function:

```

data Axis3 = X | Y | Z

fn rotate(r)(s) = extract r {
	X => px(s)
	Y => py(s)
	Z => pz(s)
}

assert rotate(X) == px
assert rotate(Y) == py
assert rotate(Z) == pz
assert rotate(X ^ Z) == hadamard	// it's back

// assert rotate(^(X, ~Y, @[1/2] Z)) == ...

```

For more documentation and examples, please check out the [tests](https://github.com/rvanasa/funqy/tree/master/tests) folder. 
