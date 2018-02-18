module Evaluator exposing (..)

import AST exposing (..)

---- Runtime evaluator ----

type Val
	= Val
	| Fn Pat Exp Val
	| State
	| NotImplemented

eval : Exp -> Val
eval exp =
	Val.NotImplemented
