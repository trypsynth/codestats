-- expect: total=10 code=5 comment=2 blank=3 shebang=0
-- simple agda fixture

module Sample where

data Bool : Set where
	true : Bool
	false : Bool

id : Bool -> Bool
