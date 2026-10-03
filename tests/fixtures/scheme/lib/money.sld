(define-library (shop money)
  (export cents->string)
  (import (scheme base))
  (begin
    (define (cents->string cents)
      (number->string (/ cents 100)))
;                           ^ d: lib/money.sld:5
; status: local
    (define (use-it) (cents->string 5))))
;                           ^ d: lib/money.sld:5
