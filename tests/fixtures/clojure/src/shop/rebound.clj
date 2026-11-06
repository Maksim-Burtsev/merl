(ns shop.rebound)
(defn grand [] 0)
(defn f23 [x] (as-> x grand (inc grand)))
;                                ^ d: src/shop/rebound.clj:3
(defn f24 [x] (as-> x y (inc (grand))))
;                             ^ d: src/shop/rebound.clj:2
(defn f29 [xs] (for [x xs :let [grand (inc x)]] grand))
;                                               ^ d: src/shop/rebound.clj:7
(defn f30 [xs] (doseq [x xs :when (pos? x) :let [y x]] (grand)))
;                                                       ^ d: src/shop/rebound.clj:2
(defn f48 [x] (try (inc x) (catch Exception grand grand)))
;                                                 ^ d: src/shop/rebound.clj:11
(defn f49 [x] (try (grand) (catch Exception e e)))
;                   ^ d: src/shop/rebound.clj:2
(defn f50 [x] (reify Object (toString [grand] (str grand))))
;                                                  ^ d: src/shop/rebound.clj:15
(defrecord Line [grand rate] Object (toString [this] (str grand rate)))
;                                                         ^ d: src/shop/rebound.clj:17
(defn f51 [] (grand))
;             ^ d: src/shop/rebound.clj:2
(defn f31 [x] (if-let [grand x] (inc grand) (grand)))
;                                    ^ d: src/shop/rebound.clj:21
;                                            ^ d: src/shop/rebound.clj:2
