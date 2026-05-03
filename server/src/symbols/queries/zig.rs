use super::LanguageConfig;

pub const SYMBOLS_QUERY: &str = r#"
(function_declaration
  name: (identifier) @function.name) @function.def

(test_declaration
  (string (string_content) @test.name)) @test.def

(variable_declaration
  (identifier) @struct.name
  (struct_declaration)) @struct.def

(variable_declaration
  (identifier) @struct.name
  (union_declaration)) @struct.def

(variable_declaration
  (identifier) @struct.name
  (opaque_declaration)) @struct.def

(variable_declaration
  (identifier) @enum.name
  (enum_declaration)) @enum.def

(variable_declaration
  (identifier) @enum.name
  (error_set_declaration)) @enum.def

(variable_declaration
  (identifier) @const.name) @const.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call_expression
  (identifier) @callee)

(call_expression
  (field_expression
    member: (identifier) @callee))

(call_expression
  (builtin_function) @callee)
"#;

pub const VARIABLES_QUERY: &str = r#"
(parameter
  name: (identifier) @var.name)

(variable_declaration
  (identifier) @var.name)

(payload
  (identifier) @var.name)
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_zig::LANGUAGE.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        // Zig tests are recognized by SymbolKind::Test (set via the test_declaration
        // capture in SYMBOLS_QUERY), not by name patterns.
        test_patterns: vec![],
    }
}
