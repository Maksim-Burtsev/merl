(ns shop.kwcafe)
(def mugful 3)
(defn kwbrew [& {:keys [mugful]}] mugful)
(defn kwserve [] (let [mugful 5] (kwbrew :mugful mugful)))
;                                         ^ d: none
;                                                 ^ d: src/shop/kwcafe.clj:4
(defn kwlook [m] (let [mugful 5] [(:mugful m) (get m ::mugful) mugful]))
;                                   ^ d: none
;                                                      ^ d: none
(defn kwtake [{:keys [mugful]}] mugful)
;                     ^ d: src/shop/kwcafe.clj:10
;                               ^ d: src/shop/kwcafe.clj:10
(defn kwmap [] (let [mugful 5] {:mugful mugful #?(:clj mugful) 1}))
;                                ^ d: none
;                                                      ^ d: src/shop/kwcafe.clj:13
