use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(class_definition
  name: (identifier) @class.name) @class.def

(object_definition
  name: (identifier) @object.name) @object.def

(trait_definition
  name: (identifier) @trait.name) @trait.def

(function_definition
  name: (identifier) @function.name) @function.def

(type_definition
  name: (type_identifier) @type.name) @type.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call_expression
  function: (identifier) @callee)

(call_expression
  function: (field_expression
    field: (identifier) @callee))
"#;

pub const VARIABLES_QUERY: &str = r#"
(val_definition
  pattern: (identifier) @var.name)

(var_definition
  pattern: (identifier) @var.name)

(parameter
  name: (identifier) @var.name)
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_scala::LANGUAGE.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![
            TestPattern::CallExpression("test"),
            TestPattern::FunctionPrefix("test"),
        ],
    }
}
