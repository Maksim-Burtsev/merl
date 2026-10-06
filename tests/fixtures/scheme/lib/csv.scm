(define csv-count
  (opt-lambda ((port 1) (in (current-input-port)))
    (read-char in)))
;              ^ d: lib/csv.scm:2
