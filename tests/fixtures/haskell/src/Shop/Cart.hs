module Shop.Cart where

import Shop.Money (Money (..), formatPrice)
--     ^ d: src/Shop/Money.hs:1
--                             ^ d: src/Shop/Money.hs:6
import qualified Data.Map as Map
import qualified Shop.Courier as C

label :: Money -> String
--       ^ d: src/Shop/Money.hs:3
--       status: Money: via import src/Shop/Money.hs
label m = go (formatPrice m)
--            ^ d: src/Shop/Money.hs:6
--            status: formatPrice: via import src/Shop/Money.hs
--        ^ d: src/Shop/Cart.hs:20
--        status: go: local
--                        ^ d: src/Shop/Cart.hs:12
--                        status: m: local
  where
    go s = "Total: " ++ s
--  ^ d: src/Shop/Cart.hs:20
--                      ^ d: src/Shop/Cart.hs:20

lookupPrice :: String -> Map.Map String Money -> Maybe Money
lookupPrice = Map.lookup
--                ^ d: none
--                status: no definition for lookup
--            ^ d: none

heaviest :: C.Courier -> Bool
--            ^ d: src/Shop/Courier.hs:5
--            status: Courier: via import src/Shop/Courier.hs
heaviest c = C.weigh c 100
--             ^ d: src/Shop/Courier.hs:7
--             status: weigh: via import src/Shop/Courier.hs
