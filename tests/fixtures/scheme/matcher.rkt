#lang racket
(define (sicp-helper x) x)
(define (sicp-extend a b) a)
(define (match pat dat dict)
  (if (null? pat) dict
      (match (cdr pat) (cdr dat)
             (sicp-helper dict))))
;             ^ d: matcher.rkt:2
(define (go p d)
  (match p d (sicp-extend 1
;             ^ d: matcher.rkt:3
                (sicp-extend 2 d))))
;                ^ d: matcher.rkt:3
(define (norm) (match-lambda (sicp-helper 1)))
;                             ^ d: matcher.rkt:2
