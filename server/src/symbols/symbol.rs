use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    Constant,
    Variable,
    Type,
    Module,
    Import,
    Test,
    Other,
}

impl SymbolKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "function" | "fn" | "func" => Some(SymbolKind::Function),
            "method" => Some(SymbolKind::Method),
            "class" => Some(SymbolKind::Class),
            "struct" => Some(SymbolKind::Struct),
            "enum" => Some(SymbolKind::Enum),
            "trait" => Some(SymbolKind::Trait),
            "interface" => Some(SymbolKind::Interface),
            "constant" | "const" => Some(SymbolKind::Constant),
            "variable" | "var" | "let" => Some(SymbolKind::Variable),
            "type" => Some(SymbolKind::Type),
            "module" | "mod" => Some(SymbolKind::Module),
            "import" | "use" => Some(SymbolKind::Import),
            "test" => Some(SymbolKind::Test),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub byte_range: (usize, usize),
    pub line_range: (usize, usize),
    pub language: crate::index::file_entry::Language,
    /// First line of the symbol (e.g. function signature).
    pub signature: String,
    /// Agent-set human-readable description.
    pub definition: Option<String>,
    /// Parent symbol name (e.g. struct for a method).
    pub parent: Option<String>,
}
