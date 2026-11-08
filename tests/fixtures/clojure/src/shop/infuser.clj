(ns shop.infuser (:require [shop.kettle :refer [steep]]))
(defn-
  steep [x] (* 2 x))
(defn soak [] (steep 1))
;              ^ d: picker src/shop/infuser.clj:2, src/shop/kettle.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2
(defn-
  ;; steep
  infuse [x] x)
(defn pour [] (infuse (steep 2)))
;              ^ d: src/shop/infuser.clj:6
;                      ^ d: picker src/shop/infuser.clj:2, src/shop/kettle.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2
