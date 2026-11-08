(ns shop.ladler (:require [shop.kettle :refer [ladle]]))
(defmethod ladle :soup [x] x)
(defn serve [x] (ladle x))
;                ^ d: src/shop/kettle.clj:3
