-- expect: total=8 code=4 comment=3 blank=1 shebang=0
-- simple haskell fixture

main :: IO ()
main = do
  putStrLn "hi"
  -- inline comment
  return ()
