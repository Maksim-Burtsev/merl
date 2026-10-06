(ns shop.letfn)
(defn twin [] 0)
(defn pair [] (letfn [(one [] (twin)) (twin [] 1)] (one)))
;                              ^ d: src/shop/letfn.clj:3
