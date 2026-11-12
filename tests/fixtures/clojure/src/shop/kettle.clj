(ns shop.kettle)
(defn steep [x] x)
(defmulti ladle :kind)
(defn decant "Pours what `steep` made." [x] x)
;                         ^ d: picker src/shop/kettle.clj:2, src/shop/infuser.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2
