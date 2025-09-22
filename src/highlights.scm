; Comments
(comment) @comment

; Keywords
[
  "fn"
  "let"
  "type"
  "mod"
  "const"
  "match"
  "None"
  "Some"
  "Left"
  "Right"
  "false"
  "true"
  "unwrap_left"
  "unwrap_right"
  "is_none"
  "unwrap"
  "assert!"
  "panic!"
  "dbg!"
  "fold"
  "for_while"
  ;; "list!"
] @keyword

; Types
(unsigned_type) @type
(builtin_alias) @type
(sum_type) @type
(option_type) @type
(tuple_type) @type
(array_type) @type
(list_type) @type
(alias_name) @type

; Function names
(function_name) @function

; Variables and identifiers
(identifier) @variable
(witness_name) @variable.special
(jet) @function

; Literals
(bin_literal) @number
(hex_literal) @number
(dec_literal) @number

; Brackets and delimiters
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
  ","
  ";"
  ":"
  "="
] @punctuation.delimiter

; Operators (if you want extra highlighting for these)
[
  ;; "+"
  ;; "-"
  ;; "*"
  ;; "/"
  "=>"
] @operator
