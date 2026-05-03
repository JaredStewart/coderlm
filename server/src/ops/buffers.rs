use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;

use crate::index::file_tree::FileTree;
use crate::symbols::SymbolTable;

#[derive(Debug, Clone, Serialize)]
pub struct Buffer {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// CRUD operations for named buffers stored per-project.
pub fn create_buffer(
    buffers: &DashMap<String, Buffer>,
    name: &str,
    content: &str,
    description: Option<&str>,
) -> Result<Buffer, String> {
    let buf = Buffer {
        name: name.to_string(),
        content: content.to_string(),
        description: description.map(|s| s.to_string()),
        created_at: Utc::now(),
    };
    buffers.insert(name.to_string(), buf.clone());
    Ok(buf)
}

pub fn from_file(
    buffers: &DashMap<String, Buffer>,
    root: &Path,
    file_tree: &Arc<FileTree>,
    name: &str,
    file: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> Result<Buffer, String> {
    if file_tree.get(file).is_none() {
        return Err(format!("File '{}' not found in index", file));
    }

    let abs_path = root.join(file);
    let source = std::fs::read_to_string(&abs_path)
        .map_err(|e| format!("Failed to read '{}': {}", file, e))?;

    let content = if start_line.is_some() || end_line.is_some() {
        let lines: Vec<&str> = source.lines().collect();
        let start = start_line.unwrap_or(0).min(lines.len());
        let end = end_line.unwrap_or(lines.len()).min(lines.len());
        lines[start..end].join("\n")
    } else {
        source
    };

    let desc = format!("Loaded from {}", file);
    create_buffer(buffers, name, &content, Some(&desc))
}

pub fn from_symbol(
    buffers: &DashMap<String, Buffer>,
    root: &Path,
    symbol_table: &Arc<SymbolTable>,
    name: &str,
    symbol_name: &str,
    file: &str,
) -> Result<Buffer, String> {
    let sym = symbol_table
        .get(file, symbol_name)
        .ok_or_else(|| format!("Symbol '{}' not found in '{}'", symbol_name, file))?;

    let abs_path = root.join(&sym.file);
    let source = std::fs::read_to_string(&abs_path)
        .map_err(|e| format!("Failed to read '{}': {}", sym.file, e))?;

    let start = sym.byte_range.0;
    let end = sym.byte_range.1.min(source.len());
    let content = source[start..end].to_string();
    let desc = format!("Symbol {} from {}", symbol_name, file);
    create_buffer(buffers, name, &content, Some(&desc))
}

pub fn get_buffer(buffers: &DashMap<String, Buffer>, name: &str) -> Result<Buffer, String> {
    buffers
        .get(name)
        .map(|r| r.value().clone())
        .ok_or_else(|| format!("Buffer '{}' not found", name))
}

pub fn peek_buffer(
    buffers: &DashMap<String, Buffer>,
    name: &str,
    start: usize,
    end: usize,
) -> Result<String, String> {
    let buf = buffers
        .get(name)
        .ok_or_else(|| format!("Buffer '{}' not found", name))?;

    let lines: Vec<&str> = buf.content.lines().collect();
    let total = lines.len();
    let start = start.min(total);
    let end = end.min(total);

    Ok(lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>6} | {}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[derive(Debug, Serialize)]
pub struct BufferSummary {
    pub name: String,
    pub description: Option<String>,
    pub size: usize,
    pub lines: usize,
    pub created_at: DateTime<Utc>,
}

pub fn list_buffers(buffers: &DashMap<String, Buffer>) -> Vec<BufferSummary> {
    buffers
        .iter()
        .map(|entry| {
            let buf = entry.value();
            BufferSummary {
                name: buf.name.clone(),
                description: buf.description.clone(),
                size: buf.content.len(),
                lines: buf.content.lines().count(),
                created_at: buf.created_at,
            }
        })
        .collect()
}

pub fn delete_buffer(buffers: &DashMap<String, Buffer>, name: &str) -> Result<(), String> {
    buffers
        .remove(name)
        .map(|_| ())
        .ok_or_else(|| format!("Buffer '{}' not found", name))
}
