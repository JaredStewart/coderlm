use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::index::file_entry::FileMark;
use crate::index::file_tree::FileTree;
use crate::symbols::symbol::SymbolKind;
use crate::symbols::SymbolTable;

#[derive(Debug, Serialize)]
pub struct StructureResponse {
    pub tree: String,
    pub file_count: usize,
    pub language_breakdown: Vec<LanguageCount>,
}

#[derive(Debug, Serialize)]
pub struct LanguageCount {
    pub language: String,
    pub count: usize,
}

/// A structure response with level-of-detail symbol information.
#[derive(Debug, Serialize)]
pub struct DetailedStructureResponse {
    pub tree: String,
    pub file_count: usize,
    pub language_breakdown: Vec<LanguageCount>,
    /// detail level: 0=tree only, 1=+symbols, 2=+signatures, 3=+source
    pub detail: u8,
    /// Per-file symbol info (only present when detail >= 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_symbols: Option<Vec<FileSymbolInfo>>,
}

#[derive(Debug, Serialize)]
pub struct FileSymbolInfo {
    pub file: String,
    pub symbols: Vec<SymbolSummary>,
}

#[derive(Debug, Serialize)]
pub struct SymbolSummary {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    pub line: usize,
    /// Full source (only present at detail level 3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub fn get_structure(file_tree: &Arc<FileTree>, depth: usize) -> StructureResponse {
    let tree = file_tree.render_tree(depth);
    let file_count = file_tree.len();
    let breakdown = file_tree
        .language_breakdown()
        .into_iter()
        .map(|b| LanguageCount {
            language: format!("{:?}", b.language).to_lowercase(),
            count: b.count,
        })
        .collect();

    StructureResponse {
        tree,
        file_count,
        language_breakdown: breakdown,
    }
}

/// Get structure with a level-of-detail parameter.
/// - L0: file tree only (same as get_structure)
/// - L1: file tree + top-level symbol names, kinds, and signatures per file
/// - L2: L1 + method signatures with parent info
/// - L3: L2 + full source of each symbol
pub fn get_structure_with_detail(
    root: &Path,
    file_tree: &Arc<FileTree>,
    symbol_table: &Arc<SymbolTable>,
    depth: usize,
    detail: u8,
) -> DetailedStructureResponse {
    let base = get_structure(file_tree, depth);

    if detail == 0 {
        return DetailedStructureResponse {
            tree: base.tree,
            file_count: base.file_count,
            language_breakdown: base.language_breakdown,
            detail: 0,
            file_symbols: None,
        };
    }

    // Collect symbols grouped by file
    let mut file_map: std::collections::BTreeMap<String, Vec<SymbolSummary>> =
        std::collections::BTreeMap::new();

    for entry in symbol_table.symbols.iter() {
        let sym = entry.value();

        // At L1, only show top-level symbols (classes, functions, constants, modules, types)
        // At L2+, include methods too
        if detail == 1 && sym.kind == SymbolKind::Method {
            continue;
        }

        let source = if detail >= 3 {
            let abs_path = root.join(&sym.file);
            std::fs::read_to_string(&abs_path)
                .ok()
                .and_then(|src| {
                    let start = sym.byte_range.0;
                    let end = sym.byte_range.1.min(src.len());
                    Some(src[start..end].to_string())
                })
        } else {
            None
        };

        let summary = SymbolSummary {
            name: sym.name.clone(),
            kind: sym.kind,
            signature: sym.signature.clone(),
            parent: if detail >= 2 { sym.parent.clone() } else { None },
            definition: sym.definition.clone(),
            line: sym.line_range.0,
            source,
        };

        file_map
            .entry(sym.file.clone())
            .or_default()
            .push(summary);
    }

    // Sort symbols within each file by line number
    for symbols in file_map.values_mut() {
        symbols.sort_by_key(|s| s.line);
    }

    let file_symbols: Vec<FileSymbolInfo> = file_map
        .into_iter()
        .map(|(file, symbols)| FileSymbolInfo { file, symbols })
        .collect();

    DetailedStructureResponse {
        tree: base.tree,
        file_count: base.file_count,
        language_breakdown: base.language_breakdown,
        detail,
        file_symbols: Some(file_symbols),
    }
}

pub fn define_file(
    file_tree: &Arc<FileTree>,
    file: &str,
    definition: &str,
) -> Result<(), String> {
    if let Some(mut entry) = file_tree.files.get_mut(file) {
        if entry.definition.is_some() {
            return Err(format!(
                "File '{}' already has a definition. Use redefine to update it.",
                file
            ));
        }
        entry.definition = Some(definition.to_string());
        Ok(())
    } else {
        Err(format!("File '{}' not found in index", file))
    }
}

pub fn redefine_file(
    file_tree: &Arc<FileTree>,
    file: &str,
    definition: &str,
) -> Result<(), String> {
    if let Some(mut entry) = file_tree.files.get_mut(file) {
        entry.definition = Some(definition.to_string());
        Ok(())
    } else {
        Err(format!("File '{}' not found in index", file))
    }
}

pub fn mark_file(
    file_tree: &Arc<FileTree>,
    file: &str,
    mark_str: &str,
) -> Result<(), String> {
    let mark = FileMark::from_str(mark_str)
        .ok_or_else(|| format!("Unknown mark type: '{}'. Valid: documentation, ignore, test, config, generated, custom", mark_str))?;

    if let Some(mut entry) = file_tree.files.get_mut(file) {
        if !entry.marks.contains(&mark) {
            entry.marks.push(mark);
        }
        Ok(())
    } else {
        Err(format!("File '{}' not found in index", file))
    }
}
