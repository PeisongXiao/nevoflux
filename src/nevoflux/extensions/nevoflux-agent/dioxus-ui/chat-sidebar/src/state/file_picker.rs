/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! File picker state for native file dialog integration

/// A file picked via native file dialog
#[derive(Debug, Clone, PartialEq)]
pub struct PickedFile {
    /// Absolute path to the file or directory
    pub path: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
    /// Last modified timestamp (Unix epoch)
    pub modified: Option<u64>,
    /// MIME type (if detected)
    pub mime_type: Option<String>,
}

impl PickedFile {
    /// Get the file name from the path
    pub fn name(&self) -> &str {
        self.path
            .rsplit('/')
            .next()
            .or_else(|| self.path.rsplit('\\').next())
            .unwrap_or(&self.path)
    }

    /// Get formatted size string
    pub fn formatted_size(&self) -> String {
        match self.size {
            Some(size) => {
                let size = size as f64;
                if size < 1024.0 {
                    format!("{} B", size)
                } else if size < 1024.0 * 1024.0 {
                    format!("{:.1} KB", size / 1024.0)
                } else {
                    format!("{:.1} MB", size / 1024.0 / 1024.0)
                }
            }
            None => "Directory".to_string(),
        }
    }
}

/// Pending file pick request
#[derive(Debug, Clone)]
pub struct PendingFilePick {
    pub request_id: String,
}
