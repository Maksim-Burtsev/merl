module Shop.Money (Money (..), formatPrice) where

newtype Money = Money { cents :: Int }

-- | Render cents as a price.
formatPrice :: Money -> String
formatPrice (Money c) = show (fromIntegral c / 100 :: Double)
--                                         ^ d: src/Shop/Money.hs:7
--                                         status: c: local
