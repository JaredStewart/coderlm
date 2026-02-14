/// Default ignore patterns applied on top of .gitignore rules.
/// These are directory names or file patterns that are almost never useful
/// for code intelligence.
pub const DEFAULT_IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "__pycache__",
    ".pycache",
    "target",
    "dist",
    "build",
    ".git",
    ".hg",
    ".svn",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "venv",
    ".venv",
    "env",
    ".env",
    "coverage",
    ".coverage",
    ".nyc_output",
    "htmlcov",
    ".terraform",
    ".serverless",
    ".zig-cache",
];

/// File extensions that are binary or otherwise useless for code reading.
pub const DEFAULT_IGNORE_EXTENSIONS: &[&str] = &[
    "min.js", "min.css", "pyc", "pyo", "class", "o", "so", "dylib", "dll", "exe", "a", "lib",
    "jar", "war", "ear", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "png", "jpg", "jpeg", "gif",
    "bmp", "ico", "svg", "webp", "mp3", "mp4", "avi", "mov", "wmv", "flv", "woff", "woff2", "ttf",
    "eot", "otf", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "db", "sqlite", "sqlite3",
    "lock", "map",
];

/// Maximum file size (in bytes) to index by default. Files larger than this
/// are still listed in the tree but are not parsed for symbols.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1_000_000; // 1 MB

pub fn should_ignore_dir(name: &str) -> bool {
    DEFAULT_IGNORE_DIRS.iter().any(|&d| d == name)
}

pub fn should_ignore_extension(path: &str) -> bool {
    let lower = path.to_lowercase();
    DEFAULT_IGNORE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{}", ext)))
}
