;; vim: ft=query
;; extends

;; methods
(method
  name: (identifier) @AlabasterDefinition)

;; singleton methods (def self.method)
(singleton_method
  name: (identifier) @AlabasterDefinition)

;; simple classes
(class
  name: (constant) @AlabasterDefinition)

;; modules
(module
  name: (constant) @AlabasterDefinition)

;; all constants in scope_resolution chains (handles any nesting depth)
(scope_resolution
  name: (constant) @AlabasterDefinition)
(scope_resolution
  scope: (constant) @AlabasterDefinition)

((simple_symbol) @AlabasterConstant)
((hash_key_symbol) @AlabasterConstant)

((instance_variable) @AlabasterPunct)
