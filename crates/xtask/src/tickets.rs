//! `cargo xtask ticket-lint`: validates ticket files under `tickets/{open,done}`
//! against the conventions in `tickets/README.md`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_norway::Value;

const REQUIRED_KEYS: &[&str] = &[
    "id",
    "title",
    "type",
    "milestone",
    "model",
    "effort",
    "status",
    "blocked_by",
    "nick_input",
];

const VALID_TYPES: &[&str] = &[
    "feature",
    "infra",
    "design-decision",
    "content",
    "bug",
    "tuning",
    "research",
    "playtest",
];
const VALID_MODELS: &[&str] = &["haiku-4.5", "sonnet-5", "opus-5.5", "fable-5.1"];
const VALID_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const VALID_STATUSES: &[&str] = &["todo", "in-progress", "blocked", "done"];
const VALID_NICK_INPUTS: &[&str] = &["none", "decision", "answer-first", "setup", "sign-off"];

/// Which of the two ticket folders a file lives in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Folder {
    Open,
    Done,
}

impl Folder {
    const fn dirname(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

/// The fields pulled out of a ticket's frontmatter that later checks need.
#[derive(Default)]
struct TicketFields {
    id: Option<String>,
    status: Option<String>,
    completed: Option<String>,
    blocked_by: Option<Vec<String>>,
}

/// One ticket file: where it is, what it says, and every problem found in
/// isolation (i.e. not requiring knowledge of other tickets).
struct Ticket {
    /// Path relative to the repo root, for error messages.
    display_path: String,
    folder: Folder,
    fields: TicketFields,
    errors: Vec<String>,
}

/// Runs every ticket-lint rule against `repo_root` and returns every error
/// found (empty means clean). `pr_branch` is the `--pr-branch` value, if any.
pub fn run(repo_root: &Path, pr_branch: Option<&str>) -> Vec<String> {
    let mut tickets = Vec::new();
    let mut errors = Vec::new();

    for folder in [Folder::Open, Folder::Done] {
        let dir = repo_root.join("tickets").join(folder.dirname());
        match read_ticket_files(&dir) {
            Ok(files) => {
                for (filename, content) in files {
                    let display_path = format!("tickets/{}/{filename}", folder.dirname());
                    tickets.push(parse_ticket(display_path, folder, &filename, &content));
                }
            }
            Err(e) => errors.push(format!("{}: {e}", dir.display())),
        }
    }

    for ticket in &tickets {
        for err in &ticket.errors {
            errors.push(format!("{}: {err}", ticket.display_path));
        }
    }

    errors.extend(check_unique_ids(&tickets));
    errors.extend(check_blocked_by_exists(&tickets));

    if let Some(branch) = pr_branch {
        errors.extend(check_pr_branch(&tickets, branch));
    }

    errors
}

fn read_ticket_files(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if Path::new(name).extension().is_none_or(|ext| ext != "md") || name == "README.md" {
            continue;
        }
        let content = fs::read_to_string(entry.path())?;
        files.push((name.to_string(), content));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn parse_ticket(display_path: String, folder: Folder, filename: &str, content: &str) -> Ticket {
    let mut errors = Vec::new();

    if !is_valid_ticket_filename(filename) {
        errors.push(format!(
            "filename `{filename}` must match `^\\d{{4}}-[a-z0-9-]+\\.md$`"
        ));
    }

    let mut fields = TicketFields::default();

    match extract_frontmatter(content) {
        None => errors.push("missing or malformed frontmatter (no `---` delimiters)".to_string()),
        Some(fm_text) => match serde_norway::from_str::<Value>(fm_text) {
            Err(e) => errors.push(format!("frontmatter is not valid YAML: {e}")),
            Ok(value) => {
                let Value::Mapping(map) = &value else {
                    errors.push("frontmatter must be a YAML mapping".to_string());
                    return Ticket {
                        display_path,
                        folder,
                        fields,
                        errors,
                    };
                };

                for key in REQUIRED_KEYS {
                    if !map.contains_key(Value::from(*key)) {
                        errors.push(format!("frontmatter is missing required key `{key}`"));
                    }
                }

                fields.id = scalar_string(map, "id");
                fields.status = scalar_string(map, "status");
                fields.completed = scalar_string(map, "completed");
                fields.blocked_by = string_list(map, "blocked_by");

                if map.contains_key(Value::from("blocked_by"))
                    && fields.blocked_by.is_none()
                    && !matches!(
                        map.get(Value::from("blocked_by")),
                        Some(Value::Sequence(seq)) if seq.is_empty()
                    )
                {
                    errors.push("`blocked_by` must be a list of ticket id strings".to_string());
                }

                if let Some(id) = &fields.id {
                    if let Some(stem) = filename.strip_suffix(".md") {
                        let prefix = stem.split('-').next().unwrap_or(stem);
                        if prefix != id {
                            errors.push(format!(
                                "`id: \"{id}\"` does not match filename prefix `{prefix}`"
                            ));
                        }
                    }
                } else if map.contains_key(Value::from("id")) {
                    errors.push("`id` must be a string".to_string());
                }

                check_enum(map, "type", VALID_TYPES, &mut errors);
                check_enum(map, "model", VALID_MODELS, &mut errors);
                check_enum(map, "effort", VALID_EFFORTS, &mut errors);
                check_enum(map, "status", VALID_STATUSES, &mut errors);
                check_enum(map, "nick_input", VALID_NICK_INPUTS, &mut errors);

                match folder {
                    Folder::Done => {
                        if fields.status.as_deref() != Some("done") {
                            errors.push(
                                "tickets in `tickets/done/` must have `status: done`".to_string(),
                            );
                        }
                        if fields.completed.as_deref().is_none_or(str::is_empty) {
                            errors.push(
                                "tickets in `tickets/done/` must have a `completed:` date"
                                    .to_string(),
                            );
                        }
                    }
                    Folder::Open => {
                        if fields.status.as_deref() == Some("done") {
                            errors.push(
                                "tickets in `tickets/open/` must not have `status: done`"
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        },
    }

    Ticket {
        display_path,
        folder,
        fields,
        errors,
    }
}

fn check_enum(map: &serde_norway::Mapping, key: &str, valid: &[&str], errors: &mut Vec<String>) {
    let Some(value) = scalar_string(map, key) else {
        return;
    };
    if !valid.contains(&value.as_str()) {
        errors.push(format!(
            "`{key}: {value}` is not one of {valid:?} (see tickets/README.md)"
        ));
    }
}

/// Reads `map[key]` as a plain scalar string, if present and scalar.
fn scalar_string(map: &serde_norway::Mapping, key: &str) -> Option<String> {
    match map.get(Value::from(key))? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Reads `map[key]` as a list of plain strings, if present and every element
/// is a string.
fn string_list(map: &serde_norway::Mapping, key: &str) -> Option<Vec<String>> {
    let Value::Sequence(seq) = map.get(Value::from(key))? else {
        return None;
    };
    seq.iter()
        .map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// `NNNN-slug.md`: exactly four ASCII digits, a hyphen, then one or more
/// lowercase-kebab characters.
fn is_valid_ticket_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let bytes = stem.as_bytes();
    if bytes.len() < 6 || bytes[4] != b'-' || !bytes[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let slug = &stem[5..];
    !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || c == '-')
}

/// Pulls the YAML text between the first `---` line and the next line that
/// starts with `---`.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    Some(&after_open[..end])
}

fn check_unique_ids(tickets: &[Ticket]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut errors = Vec::new();
    for ticket in tickets {
        let Some(id) = &ticket.fields.id else {
            continue;
        };
        if !seen.insert(id.as_str()) {
            errors.push(format!(
                "{}: duplicate ticket id `{id}`",
                ticket.display_path
            ));
        }
    }
    errors
}

fn check_blocked_by_exists(tickets: &[Ticket]) -> Vec<String> {
    let known_ids: HashSet<&str> = tickets
        .iter()
        .filter_map(|t| t.fields.id.as_deref())
        .collect();

    let mut errors = Vec::new();
    for ticket in tickets {
        let Some(blocked_by) = &ticket.fields.blocked_by else {
            continue;
        };
        for dep in blocked_by {
            if !known_ids.contains(dep.as_str()) {
                errors.push(format!(
                    "{}: `blocked_by` references unknown ticket id `{dep}`",
                    ticket.display_path
                ));
            }
        }
    }
    errors
}

/// With `--pr-branch t<NNNN>-<slug>`, ticket `NNNN` must be in `tickets/done/`
/// (that PR is expected to be the one archiving it). Branches that don't
/// follow the `t<NNNN>-` convention (e.g. `main`, `dependabot/...`) are
/// ignored.
fn check_pr_branch(tickets: &[Ticket], branch: &str) -> Vec<String> {
    let Some(id) = ticket_id_from_branch(branch) else {
        return Vec::new();
    };

    let ticket = tickets.iter().find(|t| t.fields.id.as_deref() == Some(id));
    match ticket {
        None => vec![format!(
            "branch `{branch}` names ticket `{id}`, but no ticket with that id exists"
        )],
        Some(t) if t.folder != Folder::Done => vec![format!(
            "branch `{branch}` names ticket `{id}`, which is not in tickets/done/ \
             (the PR for a ticket must move it to done/)"
        )],
        Some(_) => Vec::new(),
    }
}

fn ticket_id_from_branch(branch: &str) -> Option<&str> {
    let rest = branch.strip_prefix('t')?;
    let (id, _slug) = rest.split_once('-')?;
    (id.len() == 4 && id.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(id: &str, folder: Folder, status: &str, blocked_by: &[&str]) -> Ticket {
        Ticket {
            display_path: format!("tickets/{}/{id}-x.md", folder.dirname()),
            folder,
            fields: TicketFields {
                id: Some(id.to_string()),
                status: Some(status.to_string()),
                completed: None,
                blocked_by: Some(blocked_by.iter().map(|s| (*s).to_string()).collect()),
            },
            errors: Vec::new(),
        }
    }

    // -- filename --

    #[test]
    fn accepts_valid_filenames() {
        assert!(is_valid_ticket_filename("0106-branch-protection.md"));
        assert!(is_valid_ticket_filename("0001-a.md"));
        assert!(is_valid_ticket_filename("0001-a1-b2.md"));
    }

    #[test]
    fn rejects_bad_filenames() {
        assert!(!is_valid_ticket_filename("106-branch-protection.md")); // 3 digits
        assert!(!is_valid_ticket_filename("01006-branch-protection.md")); // 5 digits
        assert!(!is_valid_ticket_filename("0106_branch_protection.md")); // underscores
        assert!(!is_valid_ticket_filename("0106-Branch.md")); // uppercase
        assert!(!is_valid_ticket_filename("0106-.md")); // empty slug
        assert!(!is_valid_ticket_filename("0106-branch-protection.txt")); // wrong ext
        assert!(!is_valid_ticket_filename("readme.md"));
    }

    // -- frontmatter extraction --

    #[test]
    fn extracts_frontmatter_between_delimiters() {
        let content = "---\nid: \"1\"\nfoo: bar\n---\n\n# body\n";
        assert_eq!(extract_frontmatter(content), Some("id: \"1\"\nfoo: bar"));
    }

    #[test]
    fn missing_delimiters_returns_none() {
        assert_eq!(extract_frontmatter("# no frontmatter here\n"), None);
        assert_eq!(extract_frontmatter("id: \"1\"\n---\n"), None);
    }

    // -- full parse_ticket: required keys / values --

    const VALID_FRONTMATTER: &str = "\
---
id: \"0106\"
title: \"Test ticket\"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: [\"0102\"]
nick_input: none
completed:
---

# body
";

    #[test]
    fn accepts_well_formed_open_ticket() {
        let t = parse_ticket(
            "tickets/open/0106-x.md".to_string(),
            Folder::Open,
            "0106-x.md",
            VALID_FRONTMATTER,
        );
        assert!(t.errors.is_empty(), "unexpected errors: {:?}", t.errors);
        assert_eq!(t.fields.id.as_deref(), Some("0106"));
        assert_eq!(
            t.fields.blocked_by.as_deref(),
            Some(["0102".to_string()].as_slice())
        );
    }

    #[test]
    fn flags_missing_required_key() {
        let fm = "---\nid: \"0106\"\ntitle: x\n---\n";
        let t = parse_ticket("p".to_string(), Folder::Open, "0106-x.md", fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("missing required key `type`"))
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("missing required key `blocked_by`"))
        );
    }

    #[test]
    fn flags_malformed_yaml() {
        let fm = "---\nid: \"0106\n---\n";
        let t = parse_ticket("p".to_string(), Folder::Open, "0106-x.md", fm);
        assert!(t.errors.iter().any(|e| e.contains("not valid YAML")));
    }

    #[test]
    fn flags_bad_enum_value() {
        let fm = VALID_FRONTMATTER.replace("type: infra", "type: sidequest");
        let t = parse_ticket("p".to_string(), Folder::Open, "0106-x.md", &fm);
        assert!(t.errors.iter().any(|e| e.contains("`type: sidequest`")));
    }

    #[test]
    fn flags_id_filename_mismatch() {
        let t = parse_ticket(
            "tickets/open/0107-x.md".to_string(),
            Folder::Open,
            "0107-x.md",
            VALID_FRONTMATTER,
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("does not match filename prefix"))
        );
    }

    #[test]
    fn open_ticket_cannot_be_status_done() {
        let fm = VALID_FRONTMATTER.replace("status: todo", "status: done");
        let t = parse_ticket("p".to_string(), Folder::Open, "0106-x.md", &fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must not have `status: done`"))
        );
    }

    #[test]
    fn done_ticket_requires_status_done_and_completed_date() {
        let fm = VALID_FRONTMATTER.replace("status: todo", "status: done");
        let t = parse_ticket("p".to_string(), Folder::Done, "0106-x.md", &fm);
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must have a `completed:` date"))
        );

        let fm_ok = fm.replace("completed:", "completed: 2026-09-25");
        let t_ok = parse_ticket("p".to_string(), Folder::Done, "0106-x.md", &fm_ok);
        assert!(
            t_ok.errors.is_empty(),
            "unexpected errors: {:?}",
            t_ok.errors
        );
    }

    #[test]
    fn done_ticket_wrong_status_is_flagged() {
        let t = parse_ticket(
            "p".to_string(),
            Folder::Done,
            "0106-x.md",
            VALID_FRONTMATTER, // status: todo
        );
        assert!(
            t.errors
                .iter()
                .any(|e| e.contains("must have `status: done`"))
        );
    }

    // -- cross-file checks --

    #[test]
    fn detects_duplicate_ids() {
        let tickets = vec![
            ticket("0100", Folder::Open, "todo", &[]),
            ticket("0100", Folder::Open, "todo", &[]),
        ];
        let errors = check_unique_ids(&tickets);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate ticket id `0100`"));
    }

    #[test]
    fn detects_unknown_blocked_by() {
        let tickets = vec![ticket("0100", Folder::Open, "todo", &["0099"])];
        let errors = check_blocked_by_exists(&tickets);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown ticket id `0099`"));
    }

    #[test]
    fn allows_blocked_by_that_exists() {
        let tickets = vec![
            ticket("0100", Folder::Done, "done", &[]),
            ticket("0101", Folder::Open, "todo", &["0100"]),
        ];
        assert!(check_blocked_by_exists(&tickets).is_empty());
    }

    // -- pr-branch --

    #[test]
    fn pr_branch_ok_when_ticket_is_done() {
        let tickets = vec![ticket("0304", Folder::Done, "done", &[])];
        assert!(check_pr_branch(&tickets, "t0304-pathfinding").is_empty());
    }

    #[test]
    fn pr_branch_fails_when_ticket_still_open() {
        let tickets = vec![ticket("0304", Folder::Open, "todo", &[])];
        let errors = check_pr_branch(&tickets, "t0304-pathfinding");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not in tickets/done/"));
    }

    #[test]
    fn pr_branch_fails_when_ticket_unknown() {
        let tickets: Vec<Ticket> = Vec::new();
        let errors = check_pr_branch(&tickets, "t9999-ghost");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("no ticket with that id exists"));
    }

    #[test]
    fn pr_branch_ignores_non_ticket_branches() {
        let tickets: Vec<Ticket> = Vec::new();
        assert!(check_pr_branch(&tickets, "main").is_empty());
        assert!(check_pr_branch(&tickets, "dependabot/cargo/foo-1.2.3").is_empty());
    }

    #[test]
    fn ticket_id_from_branch_parses_prefix() {
        assert_eq!(ticket_id_from_branch("t0304-pathfinding"), Some("0304"));
        assert_eq!(ticket_id_from_branch("t304-pathfinding"), None);
        assert_eq!(ticket_id_from_branch("main"), None);
        assert_eq!(ticket_id_from_branch("t0304"), None);
    }

    // -- end to end over the real repo --

    #[test]
    fn real_repo_tickets_pass() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask");
        let errors = run(repo_root, None);
        assert!(errors.is_empty(), "ticket-lint errors: {errors:#?}");
    }
}
