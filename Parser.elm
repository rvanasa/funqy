module Parser exposing (..)

import AST exposing (..)

---- Language parser components ----

type ParseError
	= LineCol Int Int ParseError
	= InvalidSyntax String
	-- ...
	| NotImplemented

parse : String -> Result Exp ParseError
parse s =
	Err NotImplemented
