use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
; Functions
(function_declaration
  name: (identifier) @function.name) @function.def

; Constants and variables
(variable_declaration
  (identifier) @const.name) @const.def

; Structs: const X = struct { ... }
(variable_declaration
  (identifier) @struct.name
  "="
  (struct_declaration)) @struct.def

; Enums: const X = enum { ... }
(variable_declaration
  (identifier) @enum.name
  "="
  (enum_declaration)) @enum.def

; Unions: const X = union { ... }
(variable_declaration
  (identifier) @type.name
  "="
  (union_declaration)) @type.def

; Error sets: const X = error { ... }
(variable_declaration
  (identifier) @type.name
  "="
  (error_set_declaration)) @type.def

; Methods in structs
(struct_declaration
  (function_declaration
    name: (identifier) @method.name) @method.def)

; Methods in enums
(enum_declaration
  (function_declaration
    name: (identifier) @method.name) @method.def)

; Methods in unions
(union_declaration
  (function_declaration
    name: (identifier) @method.name) @method.def)
"#;

pub const CALLERS_QUERY: &str = r#"
; Direct function calls: foo(), std.debug.print()
(call_expression
  function: (identifier) @callee)

; Method calls: list.append(), value.method()
; Field expression has 'member' field (not 'operand')
(call_expression
  function: (field_expression
    member: (identifier) @callee))
"#;

pub const VARIABLES_QUERY: &str = r#"
; Local variable declarations (const/var in function bodies)
(variable_declaration
  (identifier) @var.name)

; Function parameters
(parameter
  name: (identifier) @var.name)

; For loop payload captures: for (items) |item|
(for_statement
  (payload
    (identifier) @var.name))

; While loop payload captures: while (opt) |value|
(while_statement
  (payload
    (identifier) @var.name))

; If statement payload captures: if (opt) |value|
(if_statement
  (payload
    (identifier) @var.name))

; Switch case payload captures
(switch_case
  (payload
    (identifier) @var.name))

; Error captures: foo() catch |err|
(catch_expression
  (payload
    (identifier) @var.name))
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_zig::LANGUAGE.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![TestPattern::FunctionPrefix("test")],
    }
}
