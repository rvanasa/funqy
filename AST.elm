module AST exposing (..)

---- AST types ----

-- Expression
type Exp
	= Let Pattern Exp
	-- ...
	| NotImplemented

-- Pattern
type Pat
	= Blank
	| Ident String
	| Tuple Pattern Pattern
	-- ...
	| NotImplemented
