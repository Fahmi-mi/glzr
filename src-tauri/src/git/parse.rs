use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitBadge {
    #[serde(rename = "M")]
    Modified,
    #[serde(rename = "A")]
    Added,
    #[serde(rename = "D")]
    Deleted,
    #[serde(rename = "R")]
    Renamed,
    #[serde(rename = "?")]
    Untracked,
    #[serde(rename = "!")]
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileEntry {
    pub path: String,
    pub orig_path: Option<String>,
    pub badge: GitBadge,
    pub staged: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedPorcelainStatus {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    pub staged: Vec<GitFileEntry>,
    pub unstaged: Vec<GitFileEntry>,
    pub untracked: Vec<GitFileEntry>,
    pub conflicted: Vec<GitFileEntry>,
}

fn char_to_badge(code: char) -> Option<GitBadge> {
    match code {
        'M' | 'T' => Some(GitBadge::Modified),
        'A' | 'C' => Some(GitBadge::Added),
        'D' => Some(GitBadge::Deleted),
        'R' => Some(GitBadge::Renamed),
        'U' => Some(GitBadge::Conflict),
        _ => None,
    }
}

/// Parses `git status --porcelain=v2 -z --branch` output bytes.
pub fn parse_porcelain_v2(bytes: &[u8]) -> ParsedPorcelainStatus {
    let mut result = ParsedPorcelainStatus::default();
    let mut fields = bytes.split(|&b| b == 0).peekable();

    while let Some(raw_field) = fields.next() {
        if raw_field.is_empty() {
            continue;
        }
        let entry = String::from_utf8_lossy(raw_field);

        if let Some(header) = entry.strip_prefix("# ") {
            parse_branch_header(header, &mut result);
            continue;
        }

        if let Some(rest) = entry.strip_prefix("1 ") {
            parse_ordinary_entry(rest, &mut result);
        } else if let Some(rest) = entry.strip_prefix("2 ") {
            let orig_path = fields
                .next()
                .map(|b| String::from_utf8_lossy(b).into_owned());
            parse_rename_entry(rest, orig_path, &mut result);
        } else if let Some(rest) = entry.strip_prefix("u ") {
            parse_unmerged_entry(rest, &mut result);
        } else if let Some(path) = entry.strip_prefix("? ") {
            result.untracked.push(GitFileEntry {
                path: path.to_string(),
                orig_path: None,
                badge: GitBadge::Untracked,
                staged: false,
            });
        }
    }

    result
}

fn parse_branch_header(header: &str, status: &mut ParsedPorcelainStatus) {
    if let Some(head) = header.strip_prefix("branch.head ") {
        if head == "(detached)" {
            status.detached = true;
            status.branch = None;
        } else {
            status.detached = false;
            status.branch = Some(head.to_string());
        }
    } else if let Some(upstream) = header.strip_prefix("branch.upstream ") {
        status.upstream = Some(upstream.to_string());
    } else if let Some(ab) = header.strip_prefix("branch.ab ") {
        for token in ab.split_whitespace() {
            if let Some(plus) = token.strip_prefix('+') {
                status.ahead = plus.parse::<u32>().unwrap_or(0);
            } else if let Some(minus) = token.strip_prefix('-') {
                status.behind = minus.parse::<u32>().unwrap_or(0);
            }
        }
    }
}

fn parse_ordinary_entry(rest: &str, status: &mut ParsedPorcelainStatus) {
    // Format: <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
    let mut parts = rest.splitn(8, ' ');
    let xy = parts.next().unwrap_or("..");
    let path = parts.nth(6).unwrap_or("");
    if path.is_empty() {
        return;
    }

    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');

    if let Some(badge) = char_to_badge(x) {
        status.staged.push(GitFileEntry {
            path: path.to_string(),
            orig_path: None,
            badge,
            staged: true,
        });
    }

    if let Some(badge) = char_to_badge(y) {
        status.unstaged.push(GitFileEntry {
            path: path.to_string(),
            orig_path: None,
            badge,
            staged: false,
        });
    }
}

fn parse_rename_entry(rest: &str, orig_path: Option<String>, status: &mut ParsedPorcelainStatus) {
    // Format: <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>
    let mut parts = rest.splitn(9, ' ');
    let xy = parts.next().unwrap_or("..");
    let path = parts.nth(7).unwrap_or("");
    if path.is_empty() {
        return;
    }

    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');

    if let Some(badge) = char_to_badge(x) {
        status.staged.push(GitFileEntry {
            path: path.to_string(),
            orig_path: orig_path.clone(),
            badge,
            staged: true,
        });
    }

    if let Some(badge) = char_to_badge(y) {
        status.unstaged.push(GitFileEntry {
            path: path.to_string(),
            orig_path,
            badge,
            staged: false,
        });
    }
}

fn parse_unmerged_entry(rest: &str, status: &mut ParsedPorcelainStatus) {
    // Format: <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
    let mut parts = rest.splitn(10, ' ');
    let path = parts.nth(9).unwrap_or("");
    if path.is_empty() {
        return;
    }

    status.conflicted.push(GitFileEntry {
        path: path.to_string(),
        orig_path: None,
        badge: GitBadge::Conflict,
        staged: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2_branch_and_entries() {
        let sample = b"# branch.oid 1234567890abcdef\0\
# branch.head feat/git-panel\0\
# branch.upstream origin/feat/git-panel\0\
# branch.ab +2 -1\0\
1 M. N... 100644 100644 100644 aaaaaa bbbbbb src/main.rs\0\
1 .M N... 100644 100644 100644 aaaaaa bbbbbb src/lib.rs\0\
1 MM N... 100644 100644 100644 aaaaaa bbbbbb Cargo.toml\0\
2 R. N... 100644 100644 100644 aaaaaa bbbbbb R100 src/new.rs\0src/old.rs\0\
u UU N... 100644 100644 100644 100644 aaaa bbbb cccc src/conflict.rs\0\
? untracked.txt\0";

        let parsed = parse_porcelain_v2(sample);
        assert_eq!(parsed.branch.as_deref(), Some("feat/git-panel"));
        assert_eq!(parsed.upstream.as_deref(), Some("origin/feat/git-panel"));
        assert_eq!(parsed.ahead, 2);
        assert_eq!(parsed.behind, 1);
        assert!(!parsed.detached);

        assert_eq!(parsed.staged.len(), 3);
        assert_eq!(parsed.staged[0].path, "src/main.rs");
        assert_eq!(parsed.staged[0].badge, GitBadge::Modified);
        assert_eq!(parsed.staged[1].path, "Cargo.toml");
        assert_eq!(parsed.staged[2].path, "src/new.rs");
        assert_eq!(parsed.staged[2].orig_path.as_deref(), Some("src/old.rs"));
        assert_eq!(parsed.staged[2].badge, GitBadge::Renamed);

        assert_eq!(parsed.unstaged.len(), 2);
        assert_eq!(parsed.unstaged[0].path, "src/lib.rs");
        assert_eq!(parsed.unstaged[1].path, "Cargo.toml");

        assert_eq!(parsed.conflicted.len(), 1);
        assert_eq!(parsed.conflicted[0].path, "src/conflict.rs");
        assert_eq!(parsed.conflicted[0].badge, GitBadge::Conflict);

        assert_eq!(parsed.untracked.len(), 1);
        assert_eq!(parsed.untracked[0].path, "untracked.txt");
        assert_eq!(parsed.untracked[0].badge, GitBadge::Untracked);
    }
}
