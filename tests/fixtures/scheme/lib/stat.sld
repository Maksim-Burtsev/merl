(define-library (shop stat)
  (export file-mode)
  (cond-expand
   (chibi
    (include "stat.scm"))
   (else
    (include "stat.scm")
    (begin
      (define (stat-mode x) (vector-ref x 2))))))
