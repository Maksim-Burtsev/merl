(ns example.core  ; f: 1-3
  (:require [clojure.string :as str]  ; f: 1-3
            [clojure.set :as set]))  ; f: 1-3

(def answer 42)  ; f: none

(defn greet  ; f: 7-12
  "Says hello ( to [ someone {."
  [name]  ; f: 7-12
  (let [s (str "hi " name)]  ; f: 7-12
    (println s \( \) \" \;)  ; f: 7-12
    s))  ; f: 7-12
  ; f: none
^:private  ; f: 14-16
(defn- helper [x]  ; f: 14-16
  (inc x))  ; f: 14-16

^{:doc "meta"  ; f: 18-21
  :added "1.0"}  ; f: 18-21
(defmacro unless [test & body]  ; f: 18-21
  `(if ~test nil (do ~@body)))  ; f: 18-21

'(quoted  ; f: none
  list)  ; f: none

#?(:clj  ; f: none
   (def platform :jvm))  ; f: none

#_(discarded  ; f: none
   form)  ; f: none

#_ (also  ; f: none
    discarded)  ; f: none

#(anonymous  ; f: none
  %)  ; f: none

[:vector  ; f: none
 :at-top]  ; f: none

{:map 1  ; f: none
 :at-top 2}  ; f: none

(def re #"\(not \" a [list")  ; f: none

(comment  ; f: 46-48
  (greet "x")  ; f: 46-48
  (helper 1))  ; f: 46-48

(def s "a string  ; f: 50-52
over ( lines
")  ; f: 50-52

(defn last-one [] (str "(" ")" "["  ; f: 54-55
                       "]"))  ; f: 54-55
