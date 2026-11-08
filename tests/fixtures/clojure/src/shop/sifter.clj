(ns shop.sifter (:require [core.match :refer [sieve]]))
(defn sift [x y]
  (sieve [x y]
;  ^ d: none
    [1 2] :one))
