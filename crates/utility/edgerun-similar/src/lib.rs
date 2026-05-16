//! Diff compatibility utilities used by EdgeRun Codex.

#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeTag {
    Equal,
    Delete,
    Insert,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    tag: ChangeTag,
    value: String,
}

impl Change {
    pub fn tag(&self) -> ChangeTag {
        self.tag
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextDiff {
    old: String,
    new: String,
    changes: Vec<Change>,
}

impl TextDiff {
    pub fn from_lines(old: &str, new: &str) -> Self {
        let old_lines = split_lines_keep_end(old);
        let new_lines = split_lines_keep_end(new);
        let changes = line_changes(&old_lines, &new_lines);
        Self {
            old: old.to_string(),
            new: new.to_string(),
            changes,
        }
    }

    pub fn iter_all_changes(&self) -> impl Iterator<Item = &Change> {
        self.changes.iter()
    }

    pub fn unified_diff(&self) -> UnifiedDiff<'_> {
        UnifiedDiff {
            diff: self,
            context_radius: 3,
            old_header: None,
            new_header: None,
        }
    }
}

pub struct UnifiedDiff<'a> {
    diff: &'a TextDiff,
    context_radius: usize,
    old_header: Option<String>,
    new_header: Option<String>,
}

impl UnifiedDiff<'_> {
    pub fn context_radius(mut self, radius: usize) -> Self {
        self.context_radius = radius;
        self
    }

    pub fn header(mut self, old: &str, new: &str) -> Self {
        self.old_header = Some(old.to_string());
        self.new_header = Some(new.to_string());
        self
    }
}

impl core::fmt::Display for UnifiedDiff<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.diff.old == self.diff.new {
            return Ok(());
        }

        let old_header = self.old_header.as_deref().unwrap_or("old");
        let new_header = self.new_header.as_deref().unwrap_or("new");
        writeln!(f, "--- {old_header}")?;
        writeln!(f, "+++ {new_header}")?;

        let old_total = self
            .diff
            .changes
            .iter()
            .filter(|change| change.tag != ChangeTag::Insert)
            .count();
        let new_total = self
            .diff
            .changes
            .iter()
            .filter(|change| change.tag != ChangeTag::Delete)
            .count();
        writeln!(f, "@@ -1,{old_total} +1,{new_total} @@")?;

        for change in &self.diff.changes {
            let marker = match change.tag {
                ChangeTag::Equal => ' ',
                ChangeTag::Delete => '-',
                ChangeTag::Insert => '+',
            };
            write!(f, "{marker}")?;
            f.write_str(&change.value)?;
            if !change.value.ends_with('\n') {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

fn split_lines_keep_end(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, byte) in input.bytes().enumerate() {
        if byte == b'\n' {
            lines.push(input[start..=idx].to_string());
            start = idx + 1;
        }
    }
    if start < input.len() {
        lines.push(input[start..].to_string());
    }
    lines
}

fn line_changes(old: &[String], new: &[String]) -> Vec<Change> {
    let mut lcs = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for old_idx in (0..old.len()).rev() {
        for new_idx in (0..new.len()).rev() {
            lcs[old_idx][new_idx] = if old[old_idx] == new[new_idx] {
                lcs[old_idx + 1][new_idx + 1] + 1
            } else {
                lcs[old_idx + 1][new_idx].max(lcs[old_idx][new_idx + 1])
            };
        }
    }

    let mut changes = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;
    while old_idx < old.len() && new_idx < new.len() {
        if old[old_idx] == new[new_idx] {
            changes.push(Change {
                tag: ChangeTag::Equal,
                value: old[old_idx].clone(),
            });
            old_idx += 1;
            new_idx += 1;
        } else if lcs[old_idx + 1][new_idx] >= lcs[old_idx][new_idx + 1] {
            changes.push(Change {
                tag: ChangeTag::Delete,
                value: old[old_idx].clone(),
            });
            old_idx += 1;
        } else {
            changes.push(Change {
                tag: ChangeTag::Insert,
                value: new[new_idx].clone(),
            });
            new_idx += 1;
        }
    }

    while old_idx < old.len() {
        changes.push(Change {
            tag: ChangeTag::Delete,
            value: old[old_idx].clone(),
        });
        old_idx += 1;
    }

    while new_idx < new.len() {
        changes.push(Change {
            tag: ChangeTag::Insert,
            value: new[new_idx].clone(),
        });
        new_idx += 1;
    }

    changes
}
