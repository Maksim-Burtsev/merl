(ns shop.kwuse
  (:require [clojure.spec.alpha :as s]
            [re-frame.core :as rf]
            [shop.kwspec :as spec]))
(s/valid? :shop.kwspec/cup 1)
;                      ^ d: none
(s/valid? ::spec/cup 1)
;                ^ d: none
(s/valid? :shop.kwspec/mug "")
;                      ^ d: none
(rf/dispatch [:brew])
;              ^ d: none
(rf/subscribe [::spec/level])
;                     ^ d: none
(rf/dispatch [:shop.kwspec/fill])
;                          ^ d: none
