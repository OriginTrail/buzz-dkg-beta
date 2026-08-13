//! Agent-native DKG memory proposals.

use std::collections::HashSet;

use nostr::{EventBuilder, Kind, Tag};

use crate::client::BuzzClient;
use crate::error::CliError;
use crate::validate::{read_file_or_stdin, validate_hex64, validate_uuid};
use crate::{MemoryCmd, MemoryQueryView};

const KIND_DKG_MEMORY_PROPOSAL: u16 = 40009;
const MAX_SOURCES: usize = 16;
const MAX_SPARQL_BYTES: usize = 8 * 1024;

fn bounded_json_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    max: usize,
) -> Result<&'a str, CliError> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(CliError::Usage(format!(
            "memory proposal {field} must contain 1..={max} non-control bytes"
        )));
    }
    Ok(value)
}

fn validate_v2_content(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), CliError> {
    let profiles = object
        .get("profiles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CliError::Usage("memory proposal profiles must be an array".into()))?;
    if profiles.is_empty() || profiles.len() > 3 {
        return Err(CliError::Usage(
            "memory proposal profiles must contain 1..=3 entries".into(),
        ));
    }
    let mut profile_ids = HashSet::new();
    for profile in profiles {
        let profile = profile.as_str().unwrap_or("");
        if !matches!(profile, "dkg-memory@1" | "dkg-software@1") || !profile_ids.insert(profile) {
            return Err(CliError::Usage(
                "memory proposal contains an unsupported or duplicate profile".into(),
            ));
        }
    }
    if !profile_ids.contains("dkg-memory@1") {
        return Err(CliError::Usage(
            "memory proposal profiles must include dkg-memory@1".into(),
        ));
    }
    let entities = object
        .get("entities")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CliError::Usage("memory proposal entities must be an array".into()))?;
    if entities.is_empty() || entities.len() > 100 {
        return Err(CliError::Usage(
            "memory proposal entities must contain 1..=100 entries".into(),
        ));
    }
    let mut entity_ids = HashSet::new();
    for (index, entity) in entities.iter().enumerate() {
        let entity = entity.as_object().ok_or_else(|| {
            CliError::Usage(format!(
                "memory proposal entities[{index}] must be an object"
            ))
        })?;
        let id = bounded_json_string(entity, "id", 64)?;
        if !id.chars().enumerate().all(|(position, character)| {
            (position == 0 && character.is_ascii_lowercase())
                || (position > 0
                    && (character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || character == '-'))
        }) || !entity_ids.insert(id)
        {
            return Err(CliError::Usage(format!(
                "memory proposal entities[{index}].id is invalid or duplicate"
            )));
        }
        bounded_json_string(entity, "type", 100)?;
        bounded_json_string(entity, "name", 500)?;
        let entity_type = entity
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let locator = entity.get("locator").and_then(serde_json::Value::as_object);
        let locator_kind = locator
            .and_then(|value| value.get("kind"))
            .and_then(serde_json::Value::as_str);
        if matches!(
            entity_type,
            "code:Package"
                | "code:File"
                | "code:Function"
                | "code:Class"
                | "code:Interface"
                | "code:TypeAlias"
                | "code:Enum"
        ) {
            let repository = locator
                .and_then(|value| value.get("repository"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if locator_kind != Some("code") || !repository.starts_with("https://") {
                return Err(CliError::Usage(format!(
                    "memory proposal entities[{index}] code identity requires a canonical HTTPS repository locator"
                )));
            }
        }
        if matches!(
            entity_type,
            "github:Repository" | "github:PullRequest" | "github:Issue" | "github:Commit"
        ) && locator_kind != Some("github")
        {
            return Err(CliError::Usage(format!(
                "memory proposal entities[{index}] GitHub identity requires a github locator"
            )));
        }
        if entity_type == "schema:Project" && locator_kind != Some("uri") {
            return Err(CliError::Usage(format!(
                "memory proposal entities[{index}] project identity requires a URI locator"
            )));
        }
        if entity
            .get("attributes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|attributes| attributes.len() > 20)
        {
            return Err(CliError::Usage(format!(
                "memory proposal entities[{index}].attributes exceeds 20 entries"
            )));
        }
    }
    let relations = object
        .get("relations")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CliError::Usage("memory proposal relations must be an array".into()))?;
    if relations.len() > 200 {
        return Err(CliError::Usage(
            "memory proposal relations exceeds 200 entries".into(),
        ));
    }
    for (index, relation) in relations.iter().enumerate() {
        let relation = relation.as_object().ok_or_else(|| {
            CliError::Usage(format!(
                "memory proposal relations[{index}] must be an object"
            ))
        })?;
        let subject = bounded_json_string(relation, "subject", 64)?;
        let object = bounded_json_string(relation, "object", 64)?;
        bounded_json_string(relation, "predicate", 100)?;
        if !entity_ids.contains(subject) || !entity_ids.contains(object) {
            return Err(CliError::Usage(format!(
                "memory proposal relations[{index}] references an unknown entity"
            )));
        }
    }
    Ok(())
}

pub async fn dispatch(command: MemoryCmd, client: &BuzzClient) -> Result<(), CliError> {
    match command {
        MemoryCmd::Propose {
            channel,
            source,
            input,
            dedupe_state,
            force,
        } => {
            propose(
                client,
                &channel,
                &source,
                &input,
                dedupe_state.as_deref(),
                force,
            )
            .await
        }
        MemoryCmd::Query {
            channel,
            input,
            view,
        } => query(client, &channel, &input, view).await,
    }
}

fn validate_sparql(sparql: &str) -> Result<&str, CliError> {
    let sparql = sparql.trim();
    if sparql.is_empty()
        || sparql.len() > MAX_SPARQL_BYTES
        || sparql
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(CliError::Usage(
            "SPARQL must contain 1..=8192 UTF-8 bytes and no binary control characters".into(),
        ));
    }
    Ok(sparql)
}

async fn query(
    client: &BuzzClient,
    channel: &str,
    input: &str,
    view: MemoryQueryView,
) -> Result<(), CliError> {
    validate_uuid(channel)?;
    let source = read_file_or_stdin(input)?;
    let sparql = validate_sparql(&source)?;
    let request = serde_json::json!({
        "channelId": channel,
        "operation": "semantic_query",
        "scope": { "type": "current_channel" },
        "arguments": { "sparql": sparql, "view": view.to_string() }
    });
    let response = client.post_authed_json("/api/dkg/query", &request).await?;
    println!("{response}");
    Ok(())
}

fn validate_proposal_content(content: &str) -> Result<(), CliError> {
    if content.len() > 64 * 1024 {
        return Err(CliError::Usage(
            "memory proposal JSON exceeds the 65536-byte limit".into(),
        ));
    }
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| CliError::Usage(format!("memory proposal is not valid JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| CliError::Usage("memory proposal must be a JSON object".into()))?;
    bounded_json_string(object, "summary", 1_000)?;
    match object
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => {
            let items = object
                .get("items")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| CliError::Usage("memory proposal items must be an array".into()))?;
            if items.is_empty() || items.len() > 50 {
                return Err(CliError::Usage(
                    "memory proposal items must contain 1..=50 entries".into(),
                ));
            }
        }
        Some(2) => validate_v2_content(object)?,
        _ => {
            return Err(CliError::Usage(
                "memory proposal schemaVersion must be 1 or 2".into(),
            ))
        }
    }
    Ok(())
}

/// Stable idempotency key for one proposal: the channel plus its evidence set.
///
/// Sources are lowercased, de-duplicated, and sorted first, so the key does not
/// depend on the order an agent happened to collect its evidence. Re-proposing
/// the same evidence for the same channel is the definition of a duplicate
/// write, which is what an unattended loop must never do after a retry or a
/// restart.
fn dedupe_key(channel: &str, sources: &[String]) -> String {
    let mut ids: Vec<String> = sources
        .iter()
        .map(|source| source.to_ascii_lowercase())
        .collect();
    ids.sort();
    ids.dedup();
    format!("{}:{}", channel.to_ascii_lowercase(), ids.join(","))
}

/// Read the ledger, tolerating a missing file (first run) but not a corrupt one:
/// silently treating an unreadable ledger as empty would re-enable the exact
/// double-write this flag exists to prevent.
fn read_dedupe_state(path: &std::path::Path) -> Result<HashSet<String>, CliError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => {
            return Err(CliError::Other(format!(
                "cannot read --dedupe-state {}: {error}",
                path.display()
            )))
        }
    };
    if raw.trim().is_empty() {
        return Ok(HashSet::new());
    }
    serde_json::from_str::<Vec<String>>(&raw)
        .map(HashSet::from_iter)
        .map_err(|error| {
            CliError::Other(format!(
                "--dedupe-state {} is not a JSON array of strings: {error}",
                path.display()
            ))
        })
}

/// Persist the ledger atomically (temp file in the same directory, then rename)
/// so a crash mid-write cannot truncate it into an empty, permissive state.
fn write_dedupe_state(path: &std::path::Path, keys: &HashSet<String>) -> Result<(), CliError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            CliError::Other(format!(
                "cannot create --dedupe-state directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    let mut ordered: Vec<&String> = keys.iter().collect();
    ordered.sort();
    let body = serde_json::to_string_pretty(&ordered)
        .map_err(|error| CliError::Other(format!("cannot serialize --dedupe-state: {error}")))?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, body).map_err(|error| {
        CliError::Other(format!(
            "cannot write --dedupe-state {}: {error}",
            temp.display()
        ))
    })?;
    std::fs::rename(&temp, path).map_err(|error| {
        CliError::Other(format!(
            "cannot replace --dedupe-state {}: {error}",
            path.display()
        ))
    })
}

async fn propose(
    client: &BuzzClient,
    channel: &str,
    sources: &[String],
    input: &str,
    dedupe_state: Option<&str>,
    force: bool,
) -> Result<(), CliError> {
    validate_uuid(channel)?;
    if sources.is_empty() || sources.len() > MAX_SOURCES {
        return Err(CliError::Usage(
            "--source must be supplied 1..=16 times".into(),
        ));
    }
    let mut unique = HashSet::new();
    for source in sources {
        validate_hex64(source)?;
        if !unique.insert(source.to_ascii_lowercase()) {
            return Err(CliError::Usage("duplicate --source event id".into()));
        }
    }
    // Consult the ledger before reading stdin or signing: an already-proposed
    // evidence set must cost nothing and, above all, must not reach the relay.
    let ledger_path = dedupe_state.map(std::path::PathBuf::from);
    let key = dedupe_key(channel, sources);
    let mut proposed: HashSet<String> = HashSet::new();
    if let Some(path) = ledger_path.as_deref() {
        proposed = read_dedupe_state(path)?;
        if !force && proposed.contains(&key) {
            println!(
                "{}",
                serde_json::json!({
                    "status": "skipped",
                    "reason": "already proposed for this channel and source set",
                    "channel": channel,
                    "sources": sources,
                })
            );
            return Ok(());
        }
    }
    let content = read_file_or_stdin(input)?;
    validate_proposal_content(&content)?;
    let mut tags = vec![
        Tag::parse(["h", channel])
            .map_err(|error| CliError::Other(format!("invalid channel tag: {error}")))?,
        Tag::parse(["t", "dkg-memory-proposal"])
            .map_err(|error| CliError::Other(format!("invalid proposal tag: {error}")))?,
    ];
    for source in sources {
        tags.push(
            Tag::parse(["e", source, "", "source"])
                .map_err(|error| CliError::Other(format!("invalid source tag: {error}")))?,
        );
    }
    let event = client.sign_event(
        EventBuilder::new(Kind::Custom(KIND_DKG_MEMORY_PROPOSAL), content).tags(tags),
    )?;
    let value = serde_json::to_value(event)
        .map_err(|error| CliError::Other(format!("proposal serialization failed: {error}")))?;
    let response = client.post_authed_json("/api/dkg/memory", &value).await?;
    // Record only after the relay accepted the proposal. Recording earlier would
    // let a transient failure permanently suppress a turn that never landed.
    if let Some(path) = ledger_path.as_deref() {
        proposed.insert(key);
        write_dedupe_state(path, &proposed)?;
    }
    println!("{response}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_content_requires_version_summary_and_items() {
        assert!(validate_proposal_content(
            r#"{"schemaVersion":1,"summary":"Use Oxigraph","items":[{"kind":"decision","text":"Use Oxigraph"}]}"#
        )
        .is_ok());
        assert!(
            validate_proposal_content(r#"{"schemaVersion":1,"summary":"x","items":[]}"#).is_err()
        );
        assert!(validate_proposal_content("not-json").is_err());
        assert!(validate_proposal_content(
            r#"{"schemaVersion":2,"profiles":["dkg-memory@1","dkg-software@1"],"summary":"JWT implementation","entities":[{"id":"verify-token","type":"code:Function","name":"verifyToken","locator":{"kind":"code","repository":"https://github.com/acme/api","package":"@acme/auth","path":"src/token.ts","symbol":"verifyToken","symbolKind":"function"}},{"id":"commit-one","type":"github:Commit","name":"Implement JWT","locator":{"kind":"github","repository":"acme/api","resource":"commit","id":"a1b2c3d4"}}],"relations":[{"subject":"commit-one","predicate":"github:affects","object":"verify-token"}]}"#
        )
        .is_ok());
        assert!(validate_proposal_content(
            r#"{"schemaVersion":2,"profiles":["dkg-memory@1","dkg-software@1"],"summary":"ambiguous code","entities":[{"id":"verify-token","type":"code:Function","name":"verifyToken","locator":{"kind":"code","package":"@acme/auth","path":"src/token.ts","symbol":"verifyToken","symbolKind":"function"}}],"relations":[]}"#
        )
        .is_err());
        assert!(validate_proposal_content(
            r#"{"schemaVersion":2,"profiles":["dkg-memory@1"],"summary":"x","entities":[{"id":"one","type":"memory:Entity","name":"One"}],"relations":[{"subject":"one","predicate":"memory:about","object":"missing"}]}"#
        )
        .is_err());
    }

    #[test]
    fn dedupe_key_ignores_source_order_case_and_repeats() {
        let a = dedupe_key(
            "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b",
            &["AA".repeat(32), "bb".repeat(32)],
        );
        let b = dedupe_key(
            "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b",
            &["bb".repeat(32), "aa".repeat(32), "aa".repeat(32)],
        );
        assert_eq!(a, b, "evidence order and case must not change the key");
        let other = dedupe_key(
            "1c7c2f2b-3d4e-5f6a-9b0c-1d2e3f4a5b6c",
            &["aa".repeat(32), "bb".repeat(32)],
        );
        assert_ne!(a, other, "a different channel must not collide");
    }

    #[test]
    fn dedupe_state_roundtrips_and_survives_a_missing_file() {
        let dir = std::env::temp_dir().join(format!("buzz-dedupe-{}", std::process::id()));
        let path = dir.join("state.json");
        let _ = std::fs::remove_dir_all(&dir);

        // A first run has no ledger yet; that is not an error.
        assert!(read_dedupe_state(&path)
            .expect("missing ledger reads empty")
            .is_empty());

        let mut keys = HashSet::new();
        keys.insert("channel:aaa".to_string());
        keys.insert("channel:bbb".to_string());
        write_dedupe_state(&path, &keys).expect("write ledger");
        assert_eq!(read_dedupe_state(&path).expect("read ledger"), keys);

        // No stray temp file is left behind by the atomic replace.
        assert!(!path.with_extension("tmp").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_dedupe_state_is_an_error_not_a_silent_empty_ledger() {
        let dir = std::env::temp_dir().join(format!("buzz-dedupe-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("state.json");
        std::fs::write(&path, "{ not an array }").expect("seed corrupt ledger");

        // Treating this as empty would re-enable duplicate writes.
        assert!(read_dedupe_state(&path).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn semantic_query_allows_formatting_but_rejects_binary_controls_and_oversize_input() {
        assert!(validate_sparql("SELECT ?s WHERE {\n?s <urn:p> ?o\n} LIMIT 25").is_ok());
        assert!(validate_sparql("SELECT ?s WHERE { ?s <urn:p> ?o }\0").is_err());
        assert!(validate_sparql(&"x".repeat(MAX_SPARQL_BYTES + 1)).is_err());
        assert!(validate_sparql("   ").is_err());
    }
}
