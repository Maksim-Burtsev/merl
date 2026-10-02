module Shop.Courier (Courier (..), weigh, lookup, total') where

import Prelude hiding (lookup)
import qualified Data.Map.Strict as M (Map, insert)

data Courier = Courier { courierName :: String, maxKilos :: Int }

weigh :: Courier -> Int -> Bool
weigh c kilos = go kilos
--              ^ d: src/Shop/Courier.hs:14
--              status: go: local
--                 ^ d: src/Shop/Courier.hs:9
  where
    go k = k <= maxKilos c
--              ^ d: src/Shop/Courier.hs:6

lookup :: String -> [Courier] -> Maybe Courier
lookup n = go
--         ^ d: src/Shop/Courier.hs:21
  where
    go [] = Nothing
    go (x : xs)
      | courierName x == n = Just x
--                  ^ d: src/Shop/Courier.hs:22
--                       ^ d: src/Shop/Courier.hs:18
      | otherwise = go xs
--                  ^ d: src/Shop/Courier.hs:21

total' :: [Int] -> Int
total' = foldr (+) 0

ship :: Courier -> IO ()
ship c = do
  let limit = maxKilos c
  ok <- pure (weigh c limit)
  print (ok, total' [limit])
--       ^ d: src/Shop/Courier.hs:35
--           ^ d: src/Shop/Courier.hs:29
--                   ^ d: src/Shop/Courier.hs:34

{-
retired :: Courier
-}

sortOn :: Int
sortOn = 0

insert :: Int -> [Int] -> [Int]
insert = (:)

stock = insert 1 []
--      ^ d: src/Shop/Courier.hs:48
