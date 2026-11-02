(define (table-drop! t k)
  (%table-delete! t k))
;  ^ d: lib/table.sld:5
(define (table-wipe! t)
  (%table-clear! t))
;  ^ d: none
(define (table-note t)
  (list (hash-table-delete! %table-delete!)))
;                           ^ d: lib/table.sld:5
