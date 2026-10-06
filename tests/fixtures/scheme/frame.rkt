#lang racket
(define viewer%
  (class text%
    (inherit find-snip)
    (define/public (first-snip) (find-snip 0 'after))
;                                ^ d: frame.rkt:10; want frame.rkt:4 (#733)
    (super-new)))
(define other%
  (class object%
    (define/private (find-snip x y) x)
    (super-new)))
(define item-count #f)
(define (count-items xs)
  (let loop ([xs xs] [item-count 0])
    (if (null? xs) item-count (loop (cdr xs) (+ item-count 1)))))
;                  ^ d: frame.rkt:12; want frame.rkt:14 (#733)
