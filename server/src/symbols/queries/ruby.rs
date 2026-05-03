use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(class
  name: [(constant) (scope_resolution)] @class.name) @class.def

(module
  name: [(constant) (scope_resolution)] @mod.name) @mod.def

(method
  name: (_) @method.name) @method.def

(singleton_method
  name: (_) @method.name) @method.def

(alias
  name: (_) @method.name) @method.def

(assignment
  left: (constant) @const.name) @const.def

(call
  method: (identifier) @_attr
  arguments: (argument_list (simple_symbol) @method.name)
  (#match? @_attr "^(attr_reader|attr_writer|attr_accessor|define_method|alias_method)$")) @method.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call
  method: (identifier) @callee)

(call
  method: (constant) @callee)

(yield) @callee
"#;

pub const VARIABLES_QUERY: &str = r#"
(assignment
  left: (identifier) @var.name)

(operator_assignment
  left: (identifier) @var.name)

(method_parameters (identifier) @var.name)
(block_parameters (identifier) @var.name)
(lambda_parameters (identifier) @var.name)

(optional_parameter name: (identifier) @var.name)
(keyword_parameter name: (identifier) @var.name)
(splat_parameter name: (identifier) @var.name)
(hash_splat_parameter name: (identifier) @var.name)
(block_parameter name: (identifier) @var.name)

(for
  pattern: (identifier) @var.name)

(rescue
  variable: (exception_variable (identifier) @var.name))
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
            TestPattern::CallExpression("specify"),
        ],
    }
}
