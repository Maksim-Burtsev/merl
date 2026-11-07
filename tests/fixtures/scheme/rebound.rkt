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
(define (r20 x) (match x [(list grand rate) grand] [_ (grand)]))
;                                           ^ d: rebound.rkt:20
;                                                      ^ d: rebound.rkt:2
(define (r21 x) (match x [(? valid? grand) grand] [(list 'grand a) (grand)]))
;                                          ^ d: rebound.rkt:23
;                                                                   ^ d: rebound.rkt:2
(define (r22 x) (match x [`(add ,grand) grand] [(app grand (list a)) (grand)]))
;                                       ^ d: rebound.rkt:26
;                                                                     ^ d: rebound.rkt:2
(define (r23 x)
  (match-define (list grand rate) x)
  grand)
; ^ d: rebound.rkt:30
(define (r24 x) (let () (match-define (list grand) x) grand) (grand))
;                                                     ^ d: rebound.rkt:33
;                                                             ^ d: rebound.rkt:2
(define (r25 x) (match x [(list a) "grand"] [(== grand) (grand)]))
;                                                        ^ d: rebound.rkt:2
; (match x [(list grand) grand])
(define (r26 x) (match x [(list grand) 1]) (grand))
;                                           ^ d: rebound.rkt:2
(define (r27 x) (match x [(struct grand (a)) (grand)]))
;                                             ^ d: rebound.rkt:2
(define (r28) (match-lambda [(list grand) grand] [_ (grand)]))
;                                         ^ d: rebound.rkt:43
;                                                    ^ d: rebound.rkt:2
(define (r29 x) (match x [grand grand]))
;                               ^ d: rebound.rkt:46
(define (r30 x) (match-define grand x) grand)
;                                      ^ d: rebound.rkt:48
(define (r31) (let ([grand 1] [b grand]) (list b grand)))
;                                ^ d: rebound.rkt:2
;                                                ^ d: rebound.rkt:50
