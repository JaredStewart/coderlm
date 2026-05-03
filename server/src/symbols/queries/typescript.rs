use super::{LanguageConfig, TestPattern};

pub const SYMBOLS_QUERY: &str = r#"
(function_declaration
  name: (identifier) @function.name) @function.def

(class_declaration
  name: (type_identifier) @class.name) @class.def

(method_definition
  name: (property_identifier) @method.name) @method.def

(lexical_declaration
  (variable_declarator
    name: (identifier) @const.name
    value: (arrow_function))) @const.def

(lexical_declaration
  (variable_declarator
    name: (identifier) @function.name
    value: (function_expression))) @function.def

(interface_declaration
  name: (type_identifier) @interface.name) @interface.def

(type_alias_declaration
  name: (type_identifier) @type.name) @type.def

(enum_declaration
  name: (identifier) @enum.name) @enum.def
"#;

pub const CALLERS_QUERY: &str = r#"
(call_expression
  function: (identifier) @callee)

(call_expression
  function: (member_expression
    property: (property_identifier) @callee))
"#;

pub const VARIABLES_QUERY: &str = r#"
(variable_declarator
  name: (identifier) @var.name)

(variable_declarator
  name: (object_pattern
    (shorthand_property_identifier_pattern) @var.name))

(variable_declarator
  name: (array_pattern
    (identifier) @var.name))

(for_in_statement
  left: (identifier) @var.name)

(for_in_statement
  left: (lexical_declaration
    (variable_declarator
      name: (identifier) @var.name)))

(required_parameter
  pattern: (identifier) @var.name)

(optional_parameter
  pattern: (identifier) @var.name)
"#;

pub fn config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        symbols_query: SYMBOLS_QUERY,
        callers_query: CALLERS_QUERY,
        variables_query: VARIABLES_QUERY,
        test_patterns: vec![
            TestPattern::CallExpression("it"),
            TestPattern::CallExpression("test"),
            TestPattern::CallExpression("describe"),
        ],
    }
}

pub const JS_SYMBOLS_QUERY: &str = r#"
(function_declaration
  name: (identifier) @function.name) @function.def

(class_declaration
  name: (identifier) @class.name) @class.def

(method_definition
  name: (property_identifier) @method.name) @method.def

; const/let foo = () => {}
(lexical_declaration
  (variable_declarator
    name: (identifier) @const.name
    value: (arrow_function))) @const.def

; const/let foo = function() {}
(lexical_declaration
  (variable_declarator
    name: (identifier) @function.name
    value: (function_expression))) @function.def

; var foo = () => {}
(variable_declaration
  (variable_declarator
    name: (identifier) @const.name
    value: (arrow_function))) @const.def

; var foo = function() {}
(variable_declaration
  (variable_declarator
    name: (identifier) @function.name
    value: (function_expression))) @function.def

; exports.foo = function() {} / module.exports.foo = function() {}
(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.name)
  right: (function_expression)) @function.def

; exports.foo = () => {}  / module.exports.foo = () => {}
(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.name)
  right: (arrow_function)) @function.def
"#;

pub const JS_CALLERS_QUERY: &str = r#"
(call_expression
  function: (identifier) @callee)

(call_expression
  function: (member_expression
    property: (property_identifier) @callee))
"#;

pub const JS_VARIABLES_QUERY: &str = r#"
(variable_declarator
  name: (identifier) @var.name)

(variable_declarator
  name: (object_pattern
    (shorthand_property_identifier_pattern) @var.name))

(variable_declarator
  name: (array_pattern
    (identifier) @var.name))

(for_in_statement
  left: (identifier) @var.name)

(formal_parameters
  (identifier) @var.name)
"#;

pub fn js_config() -> LanguageConfig {
    LanguageConfig {
        language: tree_sitter_javascript::LANGUAGE.into(),
        symbols_query: JS_SYMBOLS_QUERY,
        callers_query: JS_CALLERS_QUERY,
        variables_query: JS_VARIABLES_QUERY,
        test_patterns: vec![
            TestPattern::CallExpression("it"),
            TestPattern::CallExpression("test"),
            TestPattern::CallExpression("describe"),
        ],
    }
}
