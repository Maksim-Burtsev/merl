(ns shop.infuser (:require [shop.kettle :refer [steep]]))
(defn-
  steep [x] (* 2 x))
(defn soak [] (steep 1))
;              ^ d: picker src/shop/kettle.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2; want picker src/shop/infuser.clj:2, src/shop/kettle.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2 (#1431)
