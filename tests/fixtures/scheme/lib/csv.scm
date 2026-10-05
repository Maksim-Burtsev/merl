(define csv-count
  (opt-lambda ((port 1) (in (current-input-port)))
    (read-char in)))
;              ^ d: lib/mime.scm:1; want lib/csv.scm:2 (#732)
