#lang racket
;; The shop (#428): Racket beside a Scheme library.
(require "utils.rkt" racket/list)
;             ^ d: utils.rkt:1
;                         ^ d: none

(define tax 0.2)
(define (format-price cents)
  "(define (fake-price) 0)"
  (format "$~a" (/ cents 100.0)))
(define-syntax-rule (swap! a b) (let ([tmp a]) (set! a b) (set! b tmp)))
(define-syntax unless-empty
  (syntax-rules () [(_ x body) (if (null? x) #f body)]))
(define-values (low high) (values 1 9))
(struct point (x y))
(define-record-type <order> (make-order id) order? (id order-id))
(define/contract (total items) (-> list? number?) (apply + items))
(module helpers racket)
#|
(define (hidden) 1)
#| nested |#
(define (still-hidden) 2)
|#
#;(define (ignored) 3)

(define (label cart)
  (let ([sum (total cart)] [quote-char #\"])
;                     ^ d: shop.rkt:26
; status: local
    (string-append (format-price sum) (number->string tax) (double sum))))
;                         ^ d: shop.rkt:8
;                                  ^ d: shop.rkt:27
; status: local
;                                                      ^ d: shop.rkt:7
;                                                              ^ d: utils.rkt:3

(define (run)
  (swap! low high) (unless-empty '() 1) (point 1 2) (make-order 1) (helpers)
;    ^ d: shop.rkt:11
;                         ^ d: shop.rkt:12
;         ^ d: shop.rkt:14
;              ^ d: shop.rkt:14
;                                          ^ d: shop.rkt:15
;                                                         ^ d: none
;                                                                      ^ d: shop.rkt:18
  (hidden) (still-hidden) (ignored) (fake-price) (empty? '()) (empty '()))
;     ^ d: none
;                 ^ d: none
;                             ^ d: none
;                                         ^ d: none
;                                                    ^ d: utils.rkt:4
;                                                                ^ d: utils.rkt:5
