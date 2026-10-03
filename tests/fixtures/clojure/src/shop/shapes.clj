(ns shop.shapes
  (:require [shop.money :refer [format-price]]))

(defprotocol Shape
  "Anything with an area.
  (area [this])"
  (area [this])
  (describe [this label]))

(defrecord Circle [r]
  Shape
  (area [this] (* 3.14 r r))
  (describe [this label] (str label (area this))))
;                                      ^ d: src/shop/shapes.clj:7

(deftype Square [side])

(definterface Sized (size []))

(defstruct point :x :y)

(defmulti price :kind)

(defmethod price :circle [s] (format-price (area s)))
;                                   ^ d: src/shop/money.clj:7
; status: format-price: via import src/shop/money.clj

(defmethod price :square [s] 2)

(defn build [r]
  (let [c (->Circle r)
        m (map->Circle {:r r})]
;                 ^ d: src/shop/shapes.clj:10
;               ^ d: src/shop/shapes.clj:10
    [(price c) (Square. 1) (describe m "a") (struct point 1 2) :price 'price]))
;       ^ d: picker src/shop/shapes.clj:22, src/shop/shapes.clj:24, src/shop/shapes.clj:28
;                               ^ d: src/shop/shapes.clj:8
;                                                     ^ d: src/shop/shapes.clj:20
;           ^ d: src/shop/shapes.clj:31
; status: local
;                                    ^ d: src/shop/shapes.clj:32
;                                                                        ^ d: picker src/shop/shapes.clj:22, src/shop/shapes.clj:24, src/shop/shapes.clj:28
