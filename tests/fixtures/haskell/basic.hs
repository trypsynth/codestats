-- expect: total=9 code=4 comment=3 blank=2 shebang=0
-- simple haskell fixture

main :: IO ()
main = do
  putStrLn "hi"
  -- inline comment
  return ()
