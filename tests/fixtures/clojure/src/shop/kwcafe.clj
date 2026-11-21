(ns shop.kwcafe)
(def mugful 3)
(defn kwbrew [& {:keys [mugful]}] mugful)
(defn kwserve [] (let [mugful 5] (kwbrew :mugful mugful)))
;                                         ^ d: src/shop/kwcafe.clj:3
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
(defn kwpair [cups & {:keys [mugful pot]}] [cups mugful pot])
(defn kwpairs [] (kwpair 1 :mugful 2 :pot :mugful))
;                           ^ d: src/shop/kwcafe.clj:16
;                                     ^ d: src/shop/kwcafe.clj:16
;                                          ^ d: none
(defn kwfirst [] (kwpair :mugful 2))
;                         ^ d: none
(defn kwthread [m] (-> m (kwbrew :mugful 1)))
;                                 ^ d: none
(defn kwmulti ([] 1) ([& {:keys [mugful]}] mugful))
(defn kwmore [] (kwmulti :mugful 1))
;                         ^ d: none
(defn kwdup [& {:keys [mugful]}] mugful)
