use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(function_definition
  name: (name) @function.name) @function.def

(method_declaration
  name: (name) @method.name) @method.def

(class_declaration
  name: (name) @class.name) @class.def

(interface_declaration
  name: (name) @interface.name) @interface.def

(trait_declaration
  name: (name) @trait.name) @trait.def

(enum_declaration
  name: (name) @enum.name) @enum.def

(const_declaration
  (const_element
    (name) @const.name)) @const.def
"#;

pub const CALLERS_QUERY: &str = r#"
(function_call_expression
  function: (name) @callee)

(member_call_expression
  name: (name) @callee)

(scoped_call_expression
  name: (name) @callee)
"#;

pub const VARIABLES_QUERY: &str = r#"
(simple_parameter
  name: (variable_name (name) @var.name))

(property_promotion_parameter
  name: (variable_name (name) @var.name))

(variadic_parameter
  name: (variable_name (name) @var.name))

(assignment_expression
  left: (variable_name (name) @var.name))

(foreach_statement
  (variable_name (name) @var.name))

(catch_clause
  (variable_name (name) @var.name))

(static_variable_declaration
  (variable_name (name) @var.name))
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_php::LANGUAGE_PHP.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![
            TestPattern::FunctionPrefix("test"),
            TestPattern::Attribute("Test"),
        ],
    }
}
