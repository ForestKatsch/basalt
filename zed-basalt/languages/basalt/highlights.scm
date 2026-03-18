; Keywords
[
  "fn"
  "let"
  "mut"
  "return"
  "if"
  "else"
  "match"
  "for"
  "in"
  "while"
  "loop"
  "break"
  "continue"
  "type"
  "guard"
  "import"
  "as"
  "is"
  "panic"
] @keyword

; Literals
(integer_literal) @number
(float_literal) @number
(boolean_literal) @boolean
(nil_literal) @constant.builtin
(string_literal) @string
(string_content) @string
(escape_sequence) @string.escape
(string_interpolation) @embedded

; Types
(primitive_type) @type.builtin
(type_identifier) @type

; Functions
(function_definition
  name: (identifier) @function)
(method_definition
  name: (identifier) @function)
(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_access_expression
    field: (identifier) @function))
(method_call_expression
  method: (identifier) @function)
(lambda_expression "fn" @keyword)

; Variables and parameters
(parameter
  name: (identifier) @variable.parameter)
(let_declaration
  name: (identifier) @variable)
(for_expression
  variable: (identifier) @variable)
(guard_expression
  binding: (identifier) @variable)
(identifier) @variable

; Fields and properties
(field_definition
  name: (identifier) @property)
(field_access_expression
  field: (identifier) @property)
(field_initializer
  name: (identifier) @property)

; Enum variants
(variant_definition
  name: (type_identifier) @variant)
(enum_variant_expression
  variant: (type_identifier) @variant)
(enum_pattern
  (type_identifier) @variant)

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "&&"
  "||"
  "&"
  "|"
  "^"
  "<<"
  ">>"
  "!"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "="
  ".."
  "->"
  "=>"
  "?"
] @operator

; Punctuation
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["." "," ":"] @punctuation.delimiter

; Comments
(comment) @comment

; Match patterns
(wildcard_pattern) @variable.special
(error_pattern "!" @operator)

; Type definitions
(type_definition
  name: (type_identifier) @type)
(type_alias
  name: (type_identifier) @type)
