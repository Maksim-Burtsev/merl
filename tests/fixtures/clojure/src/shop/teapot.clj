(ns shop.teapot (:require [shop.kettle :refer [steep]]))
(defn steep [x] (inc x))
(defn pour [] (steep 1))
;              ^ d: picker src/shop/teapot.clj:2, src/shop/kettle.clj:2, src/shop/stale.clj:2
