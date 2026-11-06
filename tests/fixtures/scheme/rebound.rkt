#lang racket
(define (grand) 0)
(define (r3 xs) (for ([grand xs]) (displayln grand)))
;                                            ^ d: rebound.rkt:3
(define (r4 xs) (for/list ([x xs] #:when (grand)) x))
;                                         ^ d: rebound.rkt:2
(define (r17 x) (for/fold ([acc 0]) ([grand x]) (+ acc grand)))
;                                                      ^ d: rebound.rkt:7
(define (r5 xs) (for ([grand xs]) grand) (grand))
;                                         ^ d: rebound.rkt:2
(define (r9 x) (define grand x) grand)
;                               ^ d: rebound.rkt:11
(define (r10 x) (let () (define y x) y) (grand))
;                                        ^ d: rebound.rkt:2
(define (r13 x) (define-values (grand rate) x) grand)
;                                              ^ d: rebound.rkt:15
(define (r19 x) (case-lambda [(grand) grand] [(a) (grand)]))
;                                     ^ d: rebound.rkt:17
;                                                  ^ d: rebound.rkt:2
