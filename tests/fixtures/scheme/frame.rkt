#lang racket
(define viewer%
  (class text%
    (inherit find-snip)
    (define/public (first-snip) (find-snip 0 'after))
;                                ^ d: frame.rkt:4
    (super-new)))
(define other%
  (class object%
    (define/private (find-snip x y) x)
    (super-new)))
(define item-count #f)
(define (count-items xs)
  (let loop ([xs xs] [item-count 0])
    (if (null? xs) item-count (loop (cdr xs) (+ item-count 1)))))
;                  ^ d: frame.rkt:14
(define counter%
  (class object%
    (init-field [start 0])
    (define/public (next) (+ start 1))
;                            ^ d: frame.rkt:19
    (super-new)))
