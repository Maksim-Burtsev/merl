; inherits: ecma,jsx
; extends

(identifier) @variable

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "self" "this" "super"))

((comment) @comment.documentation
  (#lua-match? @comment.documentation "^/[*][*][^*].*[*]/$"))

(call_expression
  function: (member_expression
    object: (identifier) @_obj
    property: (property_identifier) @function.builtin)
  (#eq? @_obj "console")
  (#set! priority 105))

(function_declaration
  name: (identifier) @function
  !type_parameters
  body: (_) @_body)

[
  "if"
  "else"
  "return"
] @keyword

(arguments
  .
  (string) @string.special
  .)

(pair
  key: (_)* @property
  value: (number)+ @number
  (comment)? @comment)

(ERROR) @error
(MISSING identifier) @error

((string_fragment) @injection.content
  (#offset! @injection.content 0 1 0 -1)
  (#gsub! @injection.content "\\n" " "))

"\"" @punctuation.delimiter
