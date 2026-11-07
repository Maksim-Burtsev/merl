(ns shop.strainer (:require #?(:clj [shop.kettle :refer [steep]] :cljs [shop.stale :refer [steep]])
                            #?(:clj [shop.kettle :as pot] :cljs [shop.stale :as pot])))
(defn strain [] (steep 1))
;                ^ d: picker src/shop/kettle.clj:2, src/shop/stale.clj:2, src/shop/teapot.clj:2
(defn drain [] (pot/steep 1))
;                    ^ d: none
