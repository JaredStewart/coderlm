use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tree_sitter::StreamingIterator;
use tracing::{debug, warn};

use crate::index::file_entry::Language;
use crate::index::file_tree::FileTree;
use crate::symbols::queries;
use crate::symbols::symbol::{Symbol, SymbolKind};
use crate::symbols::SymbolTable;

/// Extract symbols from a single file.
pub fn extract_symbols_from_file(
    root: &Path,
    rel_path: &str,
    language: Language,
) -> Result<Vec<Symbol>> {
    let config = match queries::get_language_config(language) {
        Some(c) => c,
        None => return Ok(Vec::new()),
    };

    let abs_path = root.join(rel_path);
    let source = std::fs::read_to_string(&abs_path)?;

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&config.language)?;

    let tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse {}", rel_path);
            return Ok(Vec::new());
        }
    };

    let query = tree_sitter::Query::new(&config.language, config.symbols_query)?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let capture_names: Vec<String> = query.capture_names().iter().map(|s| s.to_string()).collect();

    let mut symbols = Vec::new();
    let mut current_impl_type: Option<String> = None;

    while let Some(m) = matches.next() {
        let mut name: Option<String> = None;
        let mut kind: Option<SymbolKind> = None;
        let mut def_node: Option<tree_sitter::Node> = None;
        let mut parent: Option<String> = None;

        for cap in m.captures {
            let cap_name = &capture_names[cap.index as usize];
            let text = cap.node.utf8_text(source.as_bytes()).unwrap_or("");

            match cap_name.as_str() {
                "function.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Function);
                }
                "function.def" => {
                    def_node = Some(cap.node);
                }
                "method.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Method);
                    parent = current_impl_type.clone();
                }
                "method.def" => {
                    def_node = Some(cap.node);
                }
                "impl.type" => {
                    current_impl_type = Some(text.to_string());
                }
                "struct.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Struct);
                }
                "struct.def" => {
                    def_node = Some(cap.node);
                }
                "enum.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Enum);
                }
                "enum.def" => {
                    def_node = Some(cap.node);
                }
                "trait.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Trait);
                }
                "trait.def" => {
                    def_node = Some(cap.node);
                }
                "class.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Class);
                }
                "class.def" => {
                    def_node = Some(cap.node);
                }
                "interface.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Interface);
                }
                "interface.def" => {
                    def_node = Some(cap.node);
                }
                "type.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Type);
                }
                "type.def" => {
                    def_node = Some(cap.node);
                }
                "const.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Constant);
                }
                "const.def" => {
                    def_node = Some(cap.node);
                }
                "static.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Constant);
                }
                "static.def" => {
                    def_node = Some(cap.node);
                }
                "mod.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Module);
                }
                "mod.def" => {
                    def_node = Some(cap.node);
                }
                "import.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Import);
                }
                "import.def" => {
                    def_node = Some(cap.node);
                }
                "test.name" => {
                    name = Some(text.to_string());
                    kind = Some(SymbolKind::Test);
                }
                "test.def" => {
                    def_node = Some(cap.node);
                }
                _ => {}
            }
        }

        if let (Some(name), Some(kind), Some(node)) = (name, kind, def_node) {
            let start = node.start_position();
            let end = node.end_position();
            let byte_range = (node.start_byte(), node.end_byte());
            let line_range = (start.row + 1, end.row + 1); // 1-indexed

            // Extract signature (first line of the definition)
            let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
            let signature = node_text.lines().next().unwrap_or("").to_string();

            symbols.push(Symbol {
                name,
                kind,
                file: rel_path.to_string(),
                byte_range,
                line_range,
                language,
                signature,
                definition: None,
                parent,
            });
        }
    }

    // Deduplicate symbols by (file, name): keep the one with the larger byte range
    // (the more complete definition). On equal ranges, prefer specific kinds
    // (Struct/Enum/Class/etc.) over the generic Constant/Variable/Other catch-alls
    // — some grammars (e.g. Zig) emit both for the same node.
    {
        use std::collections::HashMap;
        let kind_priority = |k: SymbolKind| -> u8 {
            match k {
                SymbolKind::Other | SymbolKind::Variable | SymbolKind::Constant | SymbolKind::Import => 1,
                _ => 10,
            }
        };
        let mut best: HashMap<String, usize> = HashMap::new();
        for (i, sym) in symbols.iter().enumerate() {
            let key = format!("{}::{}", sym.file, sym.name);
            let range_size = sym.byte_range.1.saturating_sub(sym.byte_range.0);
            if let Some(&prev_idx) = best.get(&key) {
                let prev_size = symbols[prev_idx].byte_range.1.saturating_sub(symbols[prev_idx].byte_range.0);
                if range_size > prev_size {
                    best.insert(key, i);
                } else if range_size == prev_size
                    && kind_priority(sym.kind) > kind_priority(symbols[prev_idx].kind)
                {
                    best.insert(key, i);
                }
            } else {
                best.insert(key, i);
            }
        }
        let keep: std::collections::HashSet<usize> = best.into_values().collect();
        let mut idx = 0;
        symbols.retain(|_| {
            let k = idx;
            idx += 1;
            keep.contains(&k)
        });
    }

    // For Python: associate methods with their parent class by checking if a
    // function's byte_range is contained within a class's byte_range.
    if language == Language::Python {
        // Collect class ranges
        let class_ranges: Vec<(String, usize, usize)> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| (s.name.clone(), s.byte_range.0, s.byte_range.1))
            .collect();

        for sym in symbols.iter_mut() {
            if sym.kind == SymbolKind::Function && sym.parent.is_none() {
                for (class_name, start, end) in &class_ranges {
                    if sym.byte_range.0 >= *start && sym.byte_range.1 <= *end {
                        sym.kind = SymbolKind::Method;
                        sym.parent = Some(class_name.clone());
                        break;
                    }
                }
            }
        }
    }

    debug!("Extracted {} symbols from {}", symbols.len(), rel_path);
    Ok(symbols)
}

/// Extract symbols from all files in the tree. Runs on blocking threads
/// with bounded concurrency.
pub async fn extract_all_symbols(
    root: &Path,
    file_tree: &Arc<FileTree>,
    symbol_table: &Arc<SymbolTable>,
) -> Result<usize> {
    let root = root.to_path_buf();
    let file_tree = file_tree.clone();
    let symbol_table = symbol_table.clone();

    let count = tokio::task::spawn_blocking(move || -> Result<usize> {
        let mut total = 0;

        let paths: Vec<(String, Language)> = file_tree
            .files
            .iter()
            .filter(|e| e.value().language.has_tree_sitter_support())
            .map(|e| (e.key().clone(), e.value().language))
            .collect();

        for (rel_path, language) in paths {
            match extract_symbols_from_file(&root, &rel_path, language) {
                Ok(symbols) => {
                    let count = symbols.len();
                    for sym in symbols {
                        symbol_table.insert(sym);
                    }
                    // Mark file as having symbols extracted
                    if let Some(mut entry) = file_tree.files.get_mut(&rel_path) {
                        entry.symbols_extracted = true;
                    }
                    total += count;
                }
                Err(e) => {
                    debug!("Failed to extract symbols from {}: {}", rel_path, e);
                }
            }
        }

        Ok(total)
    })
    .await??;

    Ok(count)
}
