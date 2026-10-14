#lang racket/base
;; A small ledger: entries, totals and a report.
#| A block comment
   over two lines. |#
(require racket/list racket/string (only-in racket/format ~a ~r))
(provide (struct-out entry) total report)

(define max-entries 1024)
(define ratio 1.5e3)
(define half 1/2)
(define mask #xff)
(define letter #\z)
(define newline-char #\newline)
(define flags '(#t #f #true))
(define name "ledger \"main\"\n")
(define raw #"bytes")
(define pattern #rx"^[a-z]+$")
(define pattern2 #px"\\d+")
(define kw-call (sort '(3 1 2) < #:key values))
(define tags (vector 'food 'rent "misc"))
(define table (hash 'a 1 'b 2))

(struct entry (name [amount #:mutable]) #:transparent)

(define (credit? e)
  (and (entry? e) (> (entry-amount e) 0)))

(define (total entries)
  (for/fold ([sum 0]) ([e (in-list entries)])
    (+ sum (entry-amount e))))

(define (describe e)
  (cond [(credit? e) "credit"]
        [(zero? (entry-amount e)) "empty"]
        [else "debit"]))

(define (report entries #:width [width 20])
  (define n (length entries))
  (when (> n max-entries)
    (error 'report "too many entries: ~a" n))
  (for ([e entries])
    (printf "~a ~a\n" (~a (entry-name e) #:min-width width) (entry-amount e)))
  `(total ,(total entries) entries ,@(map entry-amount entries)))

(define-syntax-rule (swap! a b)
  (let ([tmp a]) (set! a b) (set! b tmp)))

#;(define (unused) 'skipped)
(define square (lambda (x) (* x x)))
(define-values (q r) (quotient/remainder 7 2))
(define text #<<END
a here string
END
  )
(match (list 1 2)
  [(list a b) (+ a b)]
  [_ 0])
(module+ test
  (require rackunit)
  (check-equal? (total (list (entry "a" 1))) 1))
