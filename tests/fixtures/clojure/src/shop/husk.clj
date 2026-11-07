(ns shop.husk)

#_ #_ (defn husk-first [] 0)
(defn husk-second [] 0)
#_ #_
(defn husk-real [] 0)
(def husk-limit 0)
(defn husk-real [] 1)
(def husk-limit 1)

(defn husk-use []
  (husk-second)
;  ^ d: none
  (husk-real)
;  ^ d: src/shop/husk.clj:8
  husk-limit)
; ^ d: src/shop/husk.clj:9
