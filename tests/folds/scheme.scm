(import (scheme base)  ; f: 1-2
        (scheme write))  ; f: 1-2

(define (greet name)  ; f: 4-7
  "Says hello ( to [ someone."
  (let ((s (string-append "hi " name)))  ; f: 4-7
    (list #\( #\) #\" #\; s)))  ; f: 4-7
  ; f: none
(define-record-type point  ; f: 9-12
  (make-point x y)  ; f: 9-12
  point?  ; f: 9-12
  (x point-x))  ; f: 9-12

#|
(define commented-out
  #| nested |#
  1)
|#

#;(define datum-comment  ; f: none
    2)  ; f: none

'(quoted  ; f: none
  list)  ; f: none

`(quasi  ; f: none
  ,list)  ; f: none

#u8(1  ; f: none
    2)  ; f: none

(define-syntax swap!  ; f: 32-37
  (syntax-rules ()  ; f: 32-37
    ((_ a b)  ; f: 32-37
     (let ((tmp a))  ; f: 32-37
       (set! a b)  ; f: 32-37
       (set! b tmp)))))  ; f: 32-37
  ; f: none
(define x 1) (define (two-forms-on-a-line)  ; f: 39-40
  2)  ; f: 39-40
