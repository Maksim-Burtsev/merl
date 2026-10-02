module Shop.Courier (Courier (..), weigh, lookup, total') where

import Prelude hiding (lookup)

data Courier = Courier { courierName :: String, maxKilos :: Int }

weigh :: Courier -> Int -> Bool
weigh c kilos = go kilos
--              ^ d: src/Shop/Courier.hs:13
--              status: go: local
--                 ^ d: src/Shop/Courier.hs:8
  where
    go k = k <= maxKilos c
--              ^ d: src/Shop/Courier.hs:5

lookup :: String -> [Courier] -> Maybe Courier
lookup n = go
--         ^ d: src/Shop/Courier.hs:20
  where
    go [] = Nothing
    go (x : xs)
      | courierName x == n = Just x
--                  ^ d: src/Shop/Courier.hs:21
--                       ^ d: src/Shop/Courier.hs:17
      | otherwise = go xs
--                  ^ d: src/Shop/Courier.hs:20

total' :: [Int] -> Int
total' = foldr (+) 0

ship :: Courier -> IO ()
ship c = do
  let limit = maxKilos c
  ok <- pure (weigh c limit)
  print (ok, total' [limit])
--       ^ d: src/Shop/Courier.hs:34
--           ^ d: src/Shop/Courier.hs:28
--                   ^ d: src/Shop/Courier.hs:33
