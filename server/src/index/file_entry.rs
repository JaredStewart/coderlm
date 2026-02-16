use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    Scala,
    C,
    Cpp,
    Ruby,
    Shell,
    Markdown,
    Json,
    Yaml,
    Toml,
    Html,
    Css,
    Sql,
    Other,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" | "pyi" => Language::Python,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "scala" | "sc" => Language::Scala,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Language::Cpp,
            "rb" => Language::Ruby,
            "sh" | "bash" | "zsh" | "fish" => Language::Shell,
            "md" | "mdx" => Language::Markdown,
            "json" | "jsonc" => Language::Json,
            "yml" | "yaml" => Language::Yaml,
            "toml" => Language::Toml,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "less" => Language::Css,
            "sql" => Language::Sql,
            _ => Language::Other,
        }
    }

    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Other)
    }

    /// Whether this language supports tree-sitter symbol extraction.
    pub fn has_tree_sitter_support(&self) -> bool {
        matches!(
            self,
            Language::Rust | Language::Python | Language::TypeScript | Language::JavaScript | Language::Go
            | Language::Java | Language::Scala
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileMark {
    Documentation,
    Ignore,
    Test,
    Config,
    Generated,
    Custom,
}

impl FileMark {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "documentation" | "doc" | "docs" => Some(FileMark::Documentation),
            "ignore" => Some(FileMark::Ignore),
            "test" | "tests" => Some(FileMark::Test),
            "config" | "configuration" => Some(FileMark::Config),
            "generated" | "gen" => Some(FileMark::Generated),
            "custom" => Some(FileMark::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub rel_path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub language: Language,
    /// Agent-set human-readable definition of what this file does.
    pub definition: Option<String>,
    /// Agent-set marks for categorization.
    pub marks: Vec<FileMark>,
    /// Whether symbols have been extracted from this file.
    pub symbols_extracted: bool,
}

impl FileEntry {
    pub fn new(rel_path: String, size: u64, modified: DateTime<Utc>) -> Self {
        let language = Language::from_path(Path::new(&rel_path));
        Self {
            rel_path,
            size,
            modified,
            language,
            definition: None,
            marks: Vec::new(),
            symbols_extracted: false,
        }
    }
}
