(ns shop.kwcafe)
(def mugful 3)
(defn kwbrew [& {:keys [mugful]}] mugful)
(defn kwserve [] (let [mugful 5] (kwbrew :mugful mugful)))
;                                         ^ d: src/shop/kwcafe.clj:4; want none (#1446)
;                                                 ^ d: src/shop/kwcafe.clj:4
