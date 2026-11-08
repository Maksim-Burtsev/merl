(ns shop.brewer (:require [shop.kettle :refer [steep]]))
(defn brew [] (steep 1))
;              ^ d: src/shop/kettle.clj:2
(defn steep-twice [] (steep (steep 1)))
