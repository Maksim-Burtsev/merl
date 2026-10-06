(define-library (shop fs)
  (export file-rdev)
  (cond-expand
   (chibi
    (include "fs.scm"))
   (chicken
    (begin
      (define (stat-rdev x) (vector-ref x 10))))))
