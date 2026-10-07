(ns shop.rec)
(defrecord Point [x y])
(defn mk [m] (Point/create m))
;             ^ d: src/shop/rec.clj:2
