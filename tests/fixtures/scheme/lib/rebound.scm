(define (tally) 0)
(define (split x)
  (receive (tally rest) (values x x)
    (display tally))
;            ^ d: lib/rebound.scm:3
  (tally))
;  ^ d: lib/rebound.scm:1
(define (careful x) (guard (tally (#t tally)) x))
;                                     ^ d: lib/rebound.scm:8
(define (careless x) (guard (e (#t (tally))) x))
;                                   ^ d: lib/rebound.scm:1
(define (count-up x) (do ((tally 0 (+ tally 1))) ((> tally x)) (display tally)))
;                                                                       ^ d: lib/rebound.scm:12
