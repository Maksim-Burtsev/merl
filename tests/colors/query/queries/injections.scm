; inherits: ecma

((comment) @injection.content
  (#set! injection.language "comment"))

((template_string) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children))

(call_expression
  function: (identifier) @_name
  (#eq? @_name "sql")
  arguments: (template_string) @injection.content
  (#set! injection.language "sql"))
