/// Tree-sitter grammar for the Basalt programming language.

module.exports = grammar({
  name: "basalt",

  extras: ($) => [/\s/, $.comment],

  inline: ($) => [$._expression],

  conflicts: ($) => [
    [$.block, $.map_literal],
    [$._block_body, $.call_expression],
    [$._block_body, $.method_call_expression, $.field_access_expression],
    [$._block_body, $.index_expression],
    [$._block_body, $.binary_expression],
    [$._block_body, $.assignment_statement],
    [$._block_body, $.as_expression],
    [$._block_body, $.is_expression],
    [$._block_body, $.range_expression],
    [$._block_body, $.try_expression],
    [$.unary_expression, $.assignment_statement],
    [$.unary_expression, $.struct_literal],
    [$.unary_expression, $.call_expression],
    [$.unary_expression, $.method_call_expression, $.field_access_expression],
    [$.unary_expression, $.index_expression],
    [$.field_access_expression, $.enum_variant_expression],

    [$._type, $._simple_type],
    [$.optional_type, $.function_type],
    [$.result_type, $.function_type],
    [$.union_type],

  ],

  precedences: ($) => [
    [
      "postfix",
      "unary",
      "power",
      "multiplicative",
      "additive",
      "shift",
      "bitand",
      "bitxor",
      "bitor",
      "comparison",
      "equality",
      "logical_and",
      "logical_or",
      "range",
      "as",
      "is",
    ],
  ],

  rules: {
    source_file: ($) => repeat($._item),

    _item: ($) =>
      choice(
        $.import_declaration,
        $.function_definition,
        $.type_definition,
        $.type_alias,
        $.let_declaration,
      ),

    // --- Declarations ---

    import_declaration: ($) =>
      seq(
        "import",
        $.string_literal,
        optional(seq("as", $.identifier)),
      ),

    function_definition: ($) =>
      seq(
        "fn",
        field("name", $.identifier),
        field("parameters", $.parameter_list),
        optional(seq("->", field("return_type", $._type))),
        field("body", $.block),
      ),

    let_declaration: ($) =>
      prec.right(seq(
        "let",
        optional("mut"),
        field("name", $.identifier),
        optional(seq(":", field("type", $._type))),
        "=",
        field("value", $._expression),
      )),

    type_definition: ($) =>
      seq(
        "type",
        field("name", $.type_identifier),
        optional(seq(":", field("parent", $.type_identifier))),
        "{",
        repeat($._type_member),
        "}",
      ),

    type_alias: ($) =>
      seq(
        "type",
        field("name", $.type_identifier),
        "=",
        field("type", $._type),
      ),

    _type_member: ($) =>
      choice($.field_definition, $.method_definition, $.variant_definition),

    field_definition: ($) =>
      seq(field("name", $.identifier), ":", field("type", $._type)),

    method_definition: ($) =>
      seq(
        "fn",
        field("name", $.identifier),
        field("parameters", $.parameter_list),
        optional(seq("->", field("return_type", $._type))),
        field("body", $.block),
      ),

    variant_definition: ($) =>
      seq(
        field("name", $.type_identifier),
        optional(
          seq("(", commaSep1($._type), ")"),
        ),
        optional(","),
      ),

    parameter_list: ($) => seq("(", commaSep($.parameter), ")"),

    parameter: ($) =>
      seq(field("name", $.identifier), ":", field("type", $._type)),

    // --- Types ---

    _type: ($) =>
      choice(
        $.primitive_type,
        $.type_identifier,
        $.qualified_type,
        $.array_type,
        $.map_type,
        $.tuple_type,
        $.optional_type,
        $.result_type,
        $.function_type,
        $.union_type,
      ),

    primitive_type: ($) =>
      choice(
        "i8", "i16", "i32", "i64",
        "u8", "u16", "u32", "u64",
        "f64", "bool", "string", "nil",
      ),

    qualified_type: ($) =>
      prec(1, seq($.identifier, ".", $.type_identifier)),

    array_type: ($) => seq("[", $._type, "]"),

    map_type: ($) => seq("[", $._type, ":", $._type, "]"),

    tuple_type: ($) => seq("(", $._type, repeat1(seq(",", $._type)), ")"),

    optional_type: ($) => prec.left(seq($._type, "?")),

    result_type: ($) => prec.left(seq($._type, "!", $._type)),

    function_type: ($) =>
      seq("fn", "(", commaSep($._type), ")", optional(seq("->", $._type))),

    union_type: ($) =>
      prec.left(seq($._simple_type, "|", $._simple_type, repeat(seq("|", $._simple_type)))),

    _simple_type: ($) =>
      choice(
        $.primitive_type,
        $.type_identifier,
        $.qualified_type,
        $.array_type,
        $.map_type,
        $.tuple_type,
        $.optional_type,
        $.result_type,
        $.function_type,
      ),

    // --- Statements ---

    block: ($) => seq("{", optional($._block_body), "}"),

    _block_body: ($) => prec.right(repeat1($._expression)),

    expression_statement: ($) => prec(-1, $._expression),

    assignment_statement: ($) =>
      prec.right(-2, seq(field("target", $._expression), "=", field("value", $._expression))),

    return_statement: ($) => prec.left(seq("return", optional($._expression))),

    break_statement: ($) => "break",

    continue_statement: ($) => "continue",

    // --- Expressions ---

    _expression: ($) =>
      choice(
        $.let_declaration,
        $.assignment_statement,
        $.return_statement,
        $.break_statement,
        $.continue_statement,
        $.integer_literal,
        $.float_literal,
        $.string_literal,
        $.boolean_literal,
        $.nil_literal,
        $.identifier,
        $.type_identifier,
        $.binary_expression,
        $.unary_expression,
        $.call_expression,
        $.method_call_expression,
        $.field_access_expression,
        $.index_expression,
        $.try_expression,
        $.as_expression,
        $.is_expression,
        $.range_expression,
        $.array_literal,
        $.map_literal,
        $.tuple_literal,
        $.struct_literal,
        $.enum_variant_expression,
        $.error_literal,
        $.if_expression,
        $.match_expression,
        $.for_expression,
        $.while_expression,
        $.loop_expression,
        $.guard_expression,
        $.lambda_expression,
        $.block,
        $.parenthesized_expression,
        $.panic_expression,
      ),

    parenthesized_expression: ($) => seq("(", $._expression, ")"),

    integer_literal: ($) =>
      token(choice(
        /0x[0-9a-fA-F]+/,
        /0b[01]+/,
        /[0-9]+/,
      )),

    float_literal: ($) => token(/[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/),

    string_literal: ($) =>
      seq(
        '"',
        repeat(choice(
          $.string_content,
          $.escape_sequence,
          $.string_interpolation,
        )),
        '"',
      ),

    string_content: ($) => token.immediate(prec(1, /[^"\\]+/)),

    escape_sequence: ($) =>
      token.immediate(seq("\\", choice("n", "t", "r", "\\", '"', "0", "("))),

    string_interpolation: ($) =>
      seq(token.immediate("\\("), $._expression, ")"),

    boolean_literal: ($) => choice("true", "false"),

    nil_literal: ($) => "nil",

    binary_expression: ($) =>
      choice(
        ...[
          ["||", "logical_or"],
          ["&&", "logical_and"],
          ["|", "bitor"],
          ["^", "bitxor"],
          ["&", "bitand"],
          ["==", "equality"],
          ["!=", "equality"],
          ["<", "comparison"],
          ["<=", "comparison"],
          [">", "comparison"],
          [">=", "comparison"],
          ["<<", "shift"],
          [">>", "shift"],
          ["+", "additive"],
          ["-", "additive"],
          ["*", "multiplicative"],
          ["/", "multiplicative"],
          ["%", "multiplicative"],
        ].map(([op, prec_name]) =>
          prec.left(
            prec_name,
            seq(
              field("left", $._expression),
              field("operator", op),
              field("right", $._expression),
            ),
          ),
        ),
        prec.right(
          "power",
          seq(
            field("left", $._expression),
            field("operator", "**"),
            field("right", $._expression),
          ),
        ),
      ),

    unary_expression: ($) =>
      prec("unary", seq(choice("-", "!"), $._expression)),

    call_expression: ($) =>
      prec.dynamic(10, prec("postfix", seq(
        field("function", $._expression),
        "(",
        commaSep($._expression),
        ")",
      ))),

    method_call_expression: ($) =>
      prec.dynamic(10, prec("postfix", seq(
        field("object", $._expression),
        ".",
        field("method", $.identifier),
        "(",
        commaSep($._expression),
        ")",
      ))),

    field_access_expression: ($) =>
      prec.dynamic(10, prec.left("postfix", seq(
        field("object", $._expression),
        ".",
        field("field", choice($.identifier, $.type_identifier)),
      ))),

    index_expression: ($) =>
      prec.dynamic(10, prec("postfix", seq(
        field("object", $._expression),
        "[",
        field("index", $._expression),
        "]",
      ))),

    try_expression: ($) => prec.dynamic(10, prec.left("postfix", seq($._expression, "?"))),

    as_expression: ($) =>
      prec.left("as", seq(
        $._expression,
        "as",
        optional("?"),
        $._type,
      )),

    is_expression: ($) =>
      prec.left("is", seq($._expression, "is", $._expression)),

    range_expression: ($) =>
      prec.left("range", seq($._expression, "..", $._expression)),

    array_literal: ($) => seq("[", commaSep($._expression), "]"),

    map_literal: ($) =>
      seq(
        "{",
        commaSep(seq($._expression, ":", $._expression)),
        "}",
      ),

    tuple_literal: ($) =>
      seq("(", $._expression, repeat1(seq(",", $._expression)), ")"),

    struct_literal: ($) =>
      prec(1, seq(
        field("type", $.type_identifier),
        "{",
        commaSep($.field_initializer),
        "}",
      )),

    field_initializer: ($) =>
      seq(field("name", $.identifier), ":", field("value", $._expression)),

    enum_variant_expression: ($) =>
      prec.left(1, seq(
        field("type", choice($.type_identifier, $.identifier)),
        ".",
        field("variant", $.type_identifier),
        optional(seq("(", commaSep($._expression), ")")),
      )),

    error_literal: ($) => seq("!(", $._expression, ")"),

    panic_expression: ($) =>
      seq("panic", "(", commaSep($._expression), ")"),

    if_expression: ($) =>
      prec.right(seq(
        "if",
        field("condition", $._expression),
        field("consequence", $.block),
        optional(seq("else", field("alternative", choice($.block, $.if_expression)))),
      )),

    match_expression: ($) =>
      seq("match", field("value", $._expression), "{", repeat($.match_arm), "}"),

    match_arm: ($) =>
      prec.right(seq(
        field("pattern", $._pattern),
        "=>",
        field("body", $._expression),
        optional(","),
      )),

    _pattern: ($) =>
      choice(
        $.wildcard_pattern,
        $.identifier,
        $.integer_literal,
        $.float_literal,
        $.string_literal,
        $.boolean_literal,
        $.nil_literal,
        $.enum_pattern,
        $.error_pattern,
        $.is_pattern,
      ),

    wildcard_pattern: ($) => "_",

    enum_pattern: ($) =>
      seq(
        optional(seq($.identifier, ".")),
        $.type_identifier,
        ".",
        $.type_identifier,
        optional(seq("(", commaSep($.identifier), ")")),
      ),

    error_pattern: ($) => seq("!", $.identifier),

    is_pattern: ($) => seq("is", choice($._type, seq($.type_identifier, ".", $.type_identifier))),

    for_expression: ($) =>
      seq(
        "for",
        field("variable", $.identifier),
        optional(seq(",", field("key", $.identifier))),
        "in",
        field("iterable", $._expression),
        field("body", $.block),
      ),

    while_expression: ($) =>
      seq("while", field("condition", $._expression), field("body", $.block)),

    loop_expression: ($) => seq("loop", field("body", $.block)),

    guard_expression: ($) =>
      seq(
        "guard",
        optional(seq("let", field("binding", $.identifier), "=")),
        field("condition", $._expression),
        "else",
        field("else_body", $.block),
      ),

    lambda_expression: ($) =>
      seq(
        "fn",
        field("parameters", $.parameter_list),
        optional(seq("->", field("return_type", $._type))),
        field("body", $.block),
      ),

    // --- Identifiers ---

    identifier: ($) => /[a-z_][a-zA-Z0-9_]*/,

    type_identifier: ($) => /[A-Z][a-zA-Z0-9_]*/,

    // --- Comments ---

    comment: ($) => token(seq("//", /.*/)),
  },
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)), optional(","));
}
