(define-library (shop pantry)
  (export pantry-wipe!)
  (import (rename (srfi 69) (hash-table-clear! %table-clear!))))
