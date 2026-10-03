;; The shop's money (#428): what cart.clj requires as `money`.
(ns shop.money
  "Prices in cents.")

(def total 0)

(defn format-price
  "Render cents as a price.
  (defn fake-price [] 0)"
  [cents]
  (format "$%.2f" (/ cents 100.0)))

(defn- ^:private ^String round-cents [cents] (Math/round (double cents)))

(defn empty-cart? [cart] (empty? (:items cart)))
;                            ^ d: none
; status: no definition for empty?

(defn empty [cart] (assoc cart :items []))

(defonce rates (atom []))

(defmacro with-cents [& body] `(do ~@body))

(comment
  (defn scratch [] (format-price 100)))

#_(defn ignored [] 1)

(defn rounded [cents]
  (round-cents (with-cents cents)))
;       ^ d: src/shop/money.clj:13
; status: round-cents: by name, 1 match
;                    ^ d: src/shop/money.clj:23
;                             ^ d: src/shop/money.clj:30
; status: cents: local
(defn later [] (scratch) (ignored) (fake-price) (empty-cart? {}) (empty {}) @rates)
;                  ^ d: none
;                            ^ d: none
;                                        ^ d: none
;                                                     ^ d: src/shop/money.clj:15
;                                                                   ^ d: src/shop/money.clj:19
;                                                                              ^ d: src/shop/money.clj:21
