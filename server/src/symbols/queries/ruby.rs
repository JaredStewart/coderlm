use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(method
  name: (identifier) @function.name) @function.def

(singleton_method
  name: (identifier) @function.name) @function.def

(class
  name: [
    (constant) @class.name
    (scope_resolution) @class.name
  ]) @class.def

(module
  name: [
    (constant) @mod.name
    (scope_resolution) @mod.name
  ]) @mod.def

(assignment
  left: (constant) @const.name) @const.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call
  method: (identifier) @callee)

(call
  receiver: (_)
  method: (identifier) @callee)
"#;

pub const VARIABLES_QUERY: &str = r#"
(assignment
  left: (identifier) @var.name)

(operator_assignment
  left: (identifier) @var.name)

(method_parameters
  (identifier) @var.name)

(method_parameters
  (optional_parameter
    name: (identifier) @var.name))

(method_parameters
  (keyword_parameter
    name: (identifier) @var.name))

(block_parameters
  (identifier) @var.name)

(lambda_parameters
  (identifier) @var.name)
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_ruby::LANGUAGE.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![
            TestPattern::FunctionPrefix("test_"),
            TestPattern::CallExpression("describe"),
            TestPattern::CallExpression("it"),
            TestPattern::CallExpression("context"),
        ],
    }
}
