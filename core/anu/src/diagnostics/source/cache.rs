use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::line::{FormattedSourceLine, format_source_line};
use super::span::Span;

#[derive(Debug, Default)]
pub struct SourceCache {
    files: RwLock<HashMap<PathBuf, Arc<[String]>>>,
}

impl SourceCache {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
        }
    }

    fn get_file_lines(&self, path: &Path) -> Option<Arc<[String]>> {
        {
            let cache = self.files.read().ok()?;
            if let Some(lines) = cache.get(path) {
                return Some(lines.clone());
            }
        }

        let content = fs::read_to_string(path).ok()?;
        let lines: Arc<[String]> = content
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into();

        if let Ok(mut cache) = self.files.write() {
            cache.insert(path.to_path_buf(), lines.clone());
        }

        Some(lines)
    }

    pub fn get_line(&self, path: &Path, line_number: usize) -> Option<String> {
        if line_number == 0 {
            return None;
        }

        let lines = self.get_file_lines(path)?;
        lines.get(line_number - 1).cloned()
    }

    pub fn get_formatted_line(&self, span: &Span) -> Option<FormattedSourceLine> {
        let raw_line = self.get_line(&span.file_path, span.line)?;
        Some(format_source_line(&raw_line, span.column, span.length))
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.files.write() {
            cache.clear();
        }
    }
}
