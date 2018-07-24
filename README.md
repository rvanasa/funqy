## FunQy: A Next-Generation Quantum Programming Language

FunQy is a novel, purely functional, architecture-agnostic quantum programming language. 
Instead of regarding algorithms in terms of [qubits](https://en.wikipedia.org/wiki/Qubit) and [logic gates](https://en.wikipedia.org/wiki/Quantum_logic_gate), 
FunQy can simulate any combination of quantum objects using a technique we call _pattern extraction_. 

Pattern extraction is a bidirectional subset of pattern matching from classical functional programming, 
with the additional quantum capability of executing all paths simultaneously. 
This abstraction provides a clear understanding of the logic and performance benefits of a particular program—instead
of executing one path at a time, pattern extraction can execute arbitrary combinations of possible inputs. 
This tends to be vastly more intuitive and scalable than the prevalent [circuit-based algorithm](https://arxiv.org/abs/1804.03719) conventions. 

### Build Requirements

- [Nightly Rust](https://doc.rust-lang.org/1.15.1/book/nightly-rust.html) `>= 1.28.0`
- [gfortran](http://laptops.eng.uci.edu/software-installation/getting-started-with-programming/fortran-tutorial?tmpl=%2Fsystem%2Fapp%2Ftemplates%2Fprint%2F&showPrintDialog=1) `>= 4.8`

### Usage Examples

Evaluate a FunQy script (expects `.fqy` file extension):
```sh
$ funqy eval path/to/ScriptFile [-o output_file.txt]
```

Start an interactive REPL session:
```sh
$ funqy repl [-h history_file.txt]
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
		F => tgt, 		//	|0⟩ ⊗ tgt => |0⟩ ⊗ tgt 
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

Here is an interesting outcome of using both qubit and qutrit values in a function:

```

data Axis3 = X | Y | Z

fn rotate(r)(s) = extract r {
	X => px(s),
	Y => py(s),
	Z => pz(s),
}

assert rotate(X) == px
assert rotate(Y) == py
assert rotate(Z) == pz
assert rotate(X ^ Z) == hadamard	// it's back

// assert rotate(^(X, ~Y, @[1/2] Z)) == ...

```

For more documentation and examples, please check out the [tests](tree/master/tests) folder. 
