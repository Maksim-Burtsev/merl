-- | A small ledger: entries, totals and a report.
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE LambdaCase #-}
module Ledger
  ( Entry (..)
  , Kind (..)
  , total
  , report
  ) where

import qualified Data.Map.Strict as Map
import Data.List (foldl', sortOn)
import Data.Maybe (fromMaybe)
import Text.Printf (printf)

{- A block comment
   over two lines. -}

maxEntries :: Int
maxEntries = 1024

ratio :: Double
ratio = 1.5e3

mask :: Int
mask = 0xff

letter :: Char
letter = 'z'

newline :: Char
newline = '\n'

data Kind = Credit | Debit
  deriving (Show, Eq, Ord)

data Entry = Entry
  { entryName :: String
  , entryAmount :: !Int
  , entryTags :: [String]
  } deriving (Show, Eq)

newtype Ledger = Ledger { unLedger :: [Entry] }

type Totals = Map.Map Kind Int

class Describe a where
  describe :: a -> String
  describe _ = "thing"

instance Describe Entry where
  describe e
    | entryAmount e > 0 = "credit"
    | entryAmount e == 0 = "empty"
    | otherwise = "debit"

kind :: Entry -> Kind
kind e = if entryAmount e >= 0 then Credit else Debit

total :: [Entry] -> Int
total = foldl' (\acc e -> acc + entryAmount e) 0

totals :: [Entry] -> Totals
totals es = Map.fromListWith (+) [(kind e, entryAmount e) | e <- es, not (null (entryName e))]

report :: [Entry] -> IO ()
report entries = do
  let n = length entries
  if n > maxEntries
    then error ("too many entries: " ++ show n)
    else pure ()
  mapM_ line (sortOn entryName entries)
  printf "total %d\n" (total entries)
  where
    line e = putStrLn $ entryName e <> " " <> show (entryAmount e)

label :: Maybe Kind -> String
label = \case
  Just Credit -> "credit"
  Just Debit -> "debit"
  Nothing -> fromMaybe "none" Nothing

safeDiv :: Int -> Int -> Either String Int
safeDiv _ 0 = Left "division by zero"
safeDiv a b = Right (a `div` b)

main :: IO ()
main = case safeDiv 7 2 of
  Left err -> putStrLn err
  Right q -> report [Entry "rent" (-900) [], Entry "salary" q ["work"]]
