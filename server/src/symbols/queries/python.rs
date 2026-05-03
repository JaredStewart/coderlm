use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(function_definition
  name: (identifier) @function.name) @function.def

(decorated_definition
  definition: (function_definition
    name: (identifier) @function.name)) @function.def

(class_definition
  name: (identifier) @class.name) @class.def

(decorated_definition
  definition: (class_definition
    name: (identifier) @class.name)) @class.def

(expression_statement
  (assignment
    left: (identifier) @const.name)) @const.def

(import_from_statement
  name: (dotted_name) @import.name) @import.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call
  function: (identifier) @callee)

(call
  function: (attribute
    attribute: (identifier) @callee))
"#;

pub const VARIABLES_QUERY: &str = r#"
(assignment
  left: (identifier) @var.name)

(assignment
  left: (pattern_list
    (identifier) @var.name))

(assignment
  left: (tuple_pattern
    (identifier) @var.name))

(for_statement
  left: (identifier) @var.name)

(for_statement
  left: (tuple_pattern
    (identifier) @var.name))

(with_item
  (as_pattern
    alias: (as_pattern_target
      (identifier) @var.name)))

(parameters
  (identifier) @var.name)

(parameters
  (default_parameter
    name: (identifier) @var.name))

(parameters
  (typed_parameter
    (identifier) @var.name))

(parameters
  (typed_default_parameter
    name: (identifier) @var.name))
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_python::LANGUAGE.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![TestPattern::FunctionPrefix("test_")],
    }
}
