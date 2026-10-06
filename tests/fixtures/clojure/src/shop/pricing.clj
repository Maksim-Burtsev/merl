(ns shop.pricing
  (:require [shop.money :as money]))

(defn quote-price [{:keys [money]}]
  (money/format-price money))
;  ^ d: src/shop/money.clj:7
