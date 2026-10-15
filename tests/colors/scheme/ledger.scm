;;; A small ledger: entries, totals and a report.
#| A block comment
   over two lines. |#
(import (scheme base) (scheme write) (only (srfi 1) fold))

(define max-entries 1024)
(define ratio 1.5e3)
(define half 1/2)
(define mask #xff)
(define letter #\z)
(define newline-char #\newline)
(define flags '(#t #f))
(define name "ledger \"main\"\n")
(define tags (vector 'food 'rent "misc"))
(define bytes #u8(1 2 3))

(define-record-type entry
  (make-entry name amount)
  entry?
  (name entry-name)
  (amount entry-amount set-entry-amount!))

(define (credit? e)
  (and (entry? e) (> (entry-amount e) 0)))

(define (total entries)
  (let loop ((rest entries) (sum 0))
    (if (null? rest)
        sum
        (loop (cdr rest) (+ sum (entry-amount (car rest)))))))

(define (describe e)
  (cond ((credit? e) "credit")
        ((zero? (entry-amount e)) => (lambda (x) "empty"))
        (else "debit")))

(define (report entries)
  (let* ((n (length entries))
         (sum (total entries)))
    (when (> n max-entries)
      (error "too many entries" n))
    (for-each
     (lambda (e)
       (display (entry-name e))
       (newline))
     entries)
    `(total ,sum entries ,@(map entry-amount entries))))

(define-syntax swap!
  (syntax-rules ()
    ((_ a b) (let ((tmp a)) (set! a b) (set! b tmp)))))

#;(define (unused) 'skipped)
(define square (lambda (x) (* x x)))
(define pair (cons 1 [list 2 3]))
(define-values (q r) (floor/ 7 2))
(case (car flags)
  ((#t) 'yes)
  (else 'no))
