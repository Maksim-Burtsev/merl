#lang racket
(define
  (kettle-boil x) (* 2 x))
(define
  #;kettle-boil
  (kettle-cool x) x)
(define (kettle-use)
  (kettle-boil 1)
;  ^ d: kettle.rkt:2
  (kettle-cool 2))
;  ^ d: kettle.rkt:4
