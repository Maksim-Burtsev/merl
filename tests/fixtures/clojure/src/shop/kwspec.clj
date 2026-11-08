(ns shop.kwspec
  (:require [clojure.spec.alpha :as s]
            [integrant.core :as ig]
            [re-frame.core :as rf]))
(defn level [] 0)
(s/def ::cup int?)
(s/def :shop.kwspec/mug string?)
(rf/reg-event-fx :brew (fn [_ _] {}))
(rf/reg-sub ::level (fn [db _] db))
(rf/reg-event-db ::fill (fn [db _] db))
(defmethod ig/init-key :shop/db [_ opts] opts)
(def config {:shop/db {:level 1}})
;                  ^ d: none
(s/valid? ::cup 1)
;           ^ d: none
(rf/dispatch [:brew])
;              ^ d: none
(rf/subscribe [::level])
;                ^ d: none
