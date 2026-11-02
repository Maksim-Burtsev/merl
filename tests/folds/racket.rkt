#lang racket  ; f: none

(require racket/string  ; f: 3-4
         racket/list)  ; f: 3-4

(define (greet name)  ; f: 6-9
  "Says hello ( to [ someone."
  (let ([s (string-append "hi " name)])  ; f: 6-9
    (list #\( #\) #\" #\; s)))  ; f: 6-9
  ; f: none
[define bracketed  ; f: 11-12
  1]  ; f: 11-12

#|
(define commented-out
  #| nested |#
  1)
|#

#;(define datum-comment  ; f: none
    2)  ; f: none

#;  ; f: none
(define also-a-datum-comment  ; f: none
  3)  ; f: none

'(quoted  ; f: none
  list)  ; f: none

#hash((a . 1)  ; f: none
      (b . 2))  ; f: none

#(a  ; f: none
  vector)  ; f: none

(define text #<<END
not ( code
  ) neither " this
END
  )  ; f: 36-40

(define-syntax-rule (swap! a b)  ; f: 42-45
  (let ([tmp a])  ; f: 42-45
    (set! a b)  ; f: 42-45
    (set! b tmp)))  ; f: 42-45

(define warning  ; f: 47-48
  "write a here string as #<<EOF")  ; f: 47-48

#| a block comment naming #<<EOF |#

(define (pour cup)  ; f: 52-54
  (let ([tea cup])  ; f: 52-54
    tea))  ; f: 52-54
