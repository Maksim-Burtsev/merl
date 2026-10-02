{-# LANGUAGE GADTs, PatternSynonyms, QuasiQuotes, TypeFamilies #-}
module Shop.Pricing where

import Shop.Money

class (Show a) => Priced a where
  rate :: a -> Int
  describe :: a -> String
  describe x = show x
--                  ^ d: src/Shop/Pricing.hs:9

data Tariff = Tariff
  { tariffName :: String
  , perKilo :: Int
  }
  deriving (Show)
--          ^ d: none

data Coupon
  = Percent Int
  | Flat Money
  deriving (Show)

instance Priced Tariff where
--       ^ d: src/Shop/Pricing.hs:6
--              ^ d: src/Shop/Pricing.hs:12
  rate t = perKilo t
--         ^ d: src/Shop/Pricing.hs:14
--                 ^ d: src/Shop/Pricing.hs:27
--                 status: t: local
  describe = tariffName
--^ d: src/Shop/Pricing.hs:8
--           ^ d: src/Shop/Pricing.hs:13

instance Priced Coupon where
  rate (Percent p) = p
--      ^ d: src/Shop/Pricing.hs:20
  rate (Flat m) = cents m
--      ^ d: src/Shop/Pricing.hs:21
--                ^ d: src/Shop/Money.hs:3
  describe c = case c of
    Percent p -> show p ++ "%"
--                    ^ d: src/Shop/Pricing.hs:42
    Flat m -> formatPrice m
--                        ^ d: src/Shop/Pricing.hs:44
--            ^ d: src/Shop/Money.hs:6

reprice :: Tariff -> Tariff
reprice t = t { perKilo = 0 }
--              ^ d: src/Shop/Pricing.hs:14

type Name = String
type family Unit a
data family Box a
newtype Weight = Weight Int

labelOf :: Name -> Box Weight -> Unit Name
--         ^ d: src/Shop/Pricing.hs:52
--                 ^ d: src/Shop/Pricing.hs:54
--                     ^ d: src/Shop/Pricing.hs:55
--                               ^ d: src/Shop/Pricing.hs:53
labelOf = undefined

pattern Free :: Coupon
pattern Free = Flat (Money 0)

isFree :: Coupon -> Bool
isFree Free = True
--     ^ d: src/Shop/Pricing.hs:65
isFree _ = False

data Expr a where
  Lit :: Int -> Expr Int
  Add :: Expr Int -> Expr Int -> Expr Int

eval :: Expr a -> a
eval (Lit n) = n
--    ^ d: src/Shop/Pricing.hs:73
eval (Add a b) = eval a + eval b

discount, surcharge :: Int -> Money -> Money
discount p (Money c) = Money (c - c * p `div` 100)
surcharge p (Money c) = Money (c + p)

weighBand 0 = "none"
weighBand n
  | n < 10 = "light"
--  ^ d: src/Shop/Pricing.hs:86
weighBand _ = "heavy"

total coupons = sum (map rate coupons)
--                       ^ d: src/Shop/Pricing.hs:7
--                       status: rate → Priced.rate (by name, 1 match)

summary :: [Coupon] -> String
summary cs = weighBand (total cs) ++ show (surcharge 1 (discount 2 (Money 0)))
--           ^ d: src/Shop/Pricing.hs:85
--                      ^ d: src/Shop/Pricing.hs:91
--                                         ^ d: src/Shop/Pricing.hs:81
--                                                      ^ d: src/Shop/Pricing.hs:81

{- A block comment, {- nested -}:
data Phantom = Phantom
-}
query = [sql|
data Ghost = Ghost
|]

ghosts :: (Phantom, Ghost)
--         ^ d: none
--                  ^ d: none
ghosts = undefined

retired = 0

m <+> n = discount m n
--                 ^ d: none
