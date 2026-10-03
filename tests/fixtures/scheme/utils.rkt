#lang racket
(provide double empty? empty)
(define (double x) (* x 2))
(define (empty? xs) (null? xs))
(define empty '())
