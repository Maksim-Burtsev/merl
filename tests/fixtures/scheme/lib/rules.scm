(define (split-pair)
  (let-values ([(head-part tail-part) (values 1 2)])
;                          ^ d: lib/rules.scm:2
    (+ head-part tail-part)))
;      ^ d: lib/rules.scm:2
(define-syntax swap-in
  (syntax-rules (into)
    ((swap-in into slot . rest) (swap-in slot rest into))))
;                  ^ d: lib/rules.scm:8
;                                             ^ d: lib/rules.scm:8
;                                                  ^ d: none
;                                ^ d: lib/rules.scm:6
(define (%raw-total) 0)
(define (raw-total) (%raw-total))
;                     ^ d: lib/rules.scm:13
(define (gauge)
  (let-syntax ((probe (syntax-rules () ((probe pin) pin))))
    (probe 1) pin))
;             ^ d: none
