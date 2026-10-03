(ns shop.cart
  (:require [shop.money :as money]
;                 ^ d: src/shop/money.clj:1
            [clojure.string :as str]))
;                   ^ d: none

(defn label [cart]
  (let [total (reduce + (:items cart))]
;                                 ^ d: src/shop/cart.clj:7
; status: cart: local
    (str/join " " ["Total:" (money/format-price total)])))
;                                        ^ d: src/shop/money.clj:7
; status: format-price: via import src/shop/money.clj
;                                                 ^ d: src/shop/cart.clj:8
; status: local
;          ^ d: none
; status: no definition for join

(defn spent [cart]
  (+ money/total (count cart)))
;            ^ d: src/shop/money.clj:5
; status: total: via import src/shop/money.clj

(defn shape-of [s]
  (money/describe s "x"))
;            ^ d: src/shop/shapes.clj:8
; status: by name, 1 match
