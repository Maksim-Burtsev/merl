(define-syntax define-checked-kettle
  (syntax-rules ()
    ((_ kettle-type kettle-make)
     (define-record-type
         kettle-type (kettle-make) kettle-pred))))
(display kettle-type)
;        ^ d: none
