(define-library (shop clock)
  (export now)
  (cond-expand
   (chibi
    (begin
      (define (now) 1)))
   (else
    (begin
      (define (now) 2)))))
