(function_definition
  "fn"
  name: (identifier) @name) @item

(method_definition
  "fn"
  name: (identifier) @name) @item

(type_definition
  "type"
  name: (type_identifier) @name) @item

(type_alias
  "type"
  name: (type_identifier) @name) @item

(let_declaration
  "let"
  name: (identifier) @name) @item
