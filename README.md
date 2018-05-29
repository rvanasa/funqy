## FunQy: A High-Level Quantum Programming Language

_Created by Albert Dayn & Ryan Vandersmith; course project for CSCI 4830 (Principles of Functional Programming) at University of Colorado Boulder_

FunQy is a concept for an ergonomic, highly expressive quantum functional programming language.

We hope to reduce the barrier to entry for aspiring quantum developers
by leveraging high-level abstractions and design patterns common to classical programming languages
in the context of quantum computing. 

In addition, we introduce a generalization of pattern matching, which we describe as "superposition extraction."
Within an `extract` block, a developer may define outputs for each possible input such that
when invoked with a superposition, the algorithm will destructure, individually invoke,
and then restructure the superposition components. FunQy exists to provide maximally convenient access to this generalization. 

Please refer to the documentation for examples. 

The language itself is implemented in Rust, and we may decide to include a supporting Python toolkit for state visualization and other auxillary features. 

This project is currently in its very early stages; we will periodically update this readme to reflect our progress. 
