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
    // Structural safety (agent-panel decision): a shared memory proposal must
    // never carry key material. This is a cheap tripwire, not a policy engine —
    // selection policy stays in the agent loop.
    let lowered = content.to_ascii_lowercase();
    if lowered.contains("nsec1")
        || lowered.contains("private_key")
        || lowered.contains("privatekey")
    {
        return Err(CliError::Usage(
            "memory proposal content appears to contain key material; refusing to propose".into(),
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

/// Two-phase ledger.
///
/// `pending` is written *before* the proposal is posted and cleared only once
/// the outcome is known. A crash between the post and the bookkeeping therefore
/// leaves a `pending` key behind, and the next run refuses to post that
/// evidence again instead of silently duplicating it. This is at-least-once
/// delivery made *visible*; exactly-once needs the relay to reject a repeated
/// (channel, evidence set), which no client-side ledger can provide.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct DedupeLedger {
    #[serde(default)]
    accepted: HashSet<String>,
    #[serde(default)]
    pending: HashSet<String>,
}

/// Exclusive advisory lock so two schedulers cannot both observe an absent key
/// and both post. `create_new` is atomic on POSIX and Windows; the lock is
/// released on every exit path, including errors, by `Drop`.
struct LedgerLock {
    path: std::path::PathBuf,
}

impl LedgerLock {
    fn acquire(ledger: &std::path::Path) -> Result<Self, CliError> {
        let path = ledger.with_extension("lock");
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| {
                CliError::Other(format!(
                    "cannot create --dedupe-state directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                let _ = writeln!(file, "pid {}", std::process::id());
                Ok(Self { path })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(CliError::Other(format!(
                    "another proposer holds {}; if no proposer is running, remove that file",
                    path.display()
                )))
            }
            Err(error) => Err(CliError::Other(format!(
                "cannot lock --dedupe-state {}: {error}",
                path.display()
            ))),
        }
    }
}

impl Drop for LedgerLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Read the ledger, tolerating a missing file (first run) but not a corrupt one:
/// silently treating an unreadable ledger as empty would re-enable the exact
/// double-write this flag exists to prevent. A bare JSON array is accepted as
/// the earlier accepted-only format.
fn read_dedupe_state(path: &std::path::Path) -> Result<DedupeLedger, CliError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DedupeLedger::default())
        }
        Err(error) => {
            return Err(CliError::Other(format!(
                "cannot read --dedupe-state {}: {error}",
                path.display()
            )))
        }
    };
    if raw.trim().is_empty() {
        return Ok(DedupeLedger::default());
    }
    if let Ok(legacy) = serde_json::from_str::<Vec<String>>(&raw) {
        return Ok(DedupeLedger {
            accepted: legacy.into_iter().collect(),
            pending: HashSet::new(),
        });
    }
    serde_json::from_str::<DedupeLedger>(&raw).map_err(|error| {
        CliError::Other(format!(
            "--dedupe-state {} is not a recognized ledger: {error}",
            path.display()
        ))
    })
}

/// Persist the ledger durably: owner-only permissions, a process-unique temp
/// file (a shared temp name races between concurrent writers), fsync before the
/// rename, and an fsync of the directory so the rename itself survives a crash.
fn write_dedupe_state(path: &std::path::Path, ledger: &DedupeLedger) -> Result<(), CliError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&parent).map_err(|error| {
        CliError::Other(format!(
            "cannot create --dedupe-state directory {}: {error}",
            parent.display()
        ))
    })?;
    let body = serde_json::to_string_pretty(ledger)
        .map_err(|error| CliError::Other(format!("cannot serialize --dedupe-state: {error}")))?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dedupe-state"),
        std::process::id()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    {
        use std::io::Write;
        let mut file = options.open(&temp).map_err(|error| {
            CliError::Other(format!(
                "cannot write --dedupe-state {}: {error}",
                temp.display()
            ))
        })?;
        file.write_all(body.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| {
                CliError::Other(format!(
                    "cannot flush --dedupe-state {}: {error}",
                    temp.display()
                ))
            })?;
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        CliError::Other(format!(
            "cannot replace --dedupe-state {}: {error}",
            path.display()
        ))
    })?;
    // Durability of the rename itself; best-effort because not every platform
    // permits opening a directory.
    if let Ok(dir) = std::fs::File::open(&parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// What the ledger decided before any observable work happens.
enum LedgerGate {
    /// No ledger configured, or `--force`: proceed without bookkeeping guards.
    Proceed,
    /// This evidence set was already accepted: report and exit successfully.
    Skip,
    /// A previous run posted without recording the outcome: fail closed unless
    /// a safe-direction read-back can prove the relay holds it.
    PendingAmbiguous,
}

/// Owns the lock, the on-disk state, and the two-phase transitions, so the
/// propose flow reads as: gate, post, record. Every transition persists before
/// it is relied on.
struct ProposalLedger {
    path: std::path::PathBuf,
    _lock: LedgerLock,
    state: DedupeLedger,
    key: String,
}

impl ProposalLedger {
    fn open(
        path: &str,
        channel: &str,
        sources: &[String],
        force: bool,
    ) -> Result<(Self, LedgerGate), CliError> {
        let path = std::path::PathBuf::from(path);
        let lock = LedgerLock::acquire(&path)?;
        let state = read_dedupe_state(&path)?;
        let key = dedupe_key(channel, sources);
        let gate = if force {
            LedgerGate::Proceed
        } else if state.accepted.contains(&key) {
            LedgerGate::Skip
        } else if state.pending.contains(&key) {
            LedgerGate::PendingAmbiguous
        } else {
            LedgerGate::Proceed
        };
        Ok((
            Self {
                path,
                _lock: lock,
                state,
                key,
            },
            gate,
        ))
    }

    /// Persist the attempt before the relay can observe it, so a crash in the
    /// post window is detectable on the next run rather than invisible.
    fn mark_pending(&mut self) -> Result<(), CliError> {
        self.state.pending.insert(self.key.clone());
        write_dedupe_state(&self.path, &self.state)
    }

    fn record_accepted(&mut self) -> Result<(), CliError> {
        self.state.pending.remove(&self.key);
        self.state.accepted.insert(self.key.clone());
        write_dedupe_state(&self.path, &self.state)
    }

    /// Classify a post failure per the relay contract (deliberated with the
    /// agent panel): clear `pending` ONLY for responses the relay guarantees
    /// are pre-ingestion; reconcile duplicate/conflict answers to `accepted`
    /// (the logical record exists); leave everything else pending so the next
    /// run fails closed instead of double-writing.
    fn record_failure(&mut self, error: &CliError) -> Result<(), CliError> {
        match error {
            // 401/403 and endpoint validation happen before any forward to the
            // DKG gateway — nothing was stored.
            CliError::Auth(_) => {
                self.state.pending.remove(&self.key);
                write_dedupe_state(&self.path, &self.state)
            }
            CliError::Relay { status, body } => {
                let body = body.to_ascii_lowercase();
                let already_exists =
                    *status == 409 || body.contains("duplicate") || body.contains("already");
                let pre_ingestion = matches!(status, 400 | 404 | 413)
                    && (body.contains("invalid")
                        || body.contains("restricted")
                        || body.contains("not found")
                        || body.contains("exceeds"));
                if already_exists {
                    self.state.pending.remove(&self.key);
                    self.state.accepted.insert(self.key.clone());
                    write_dedupe_state(&self.path, &self.state)
                } else if pre_ingestion {
                    self.state.pending.remove(&self.key);
                    write_dedupe_state(&self.path, &self.state)
                } else {
                    // Unfamiliar status: outcome unclassified, fail closed.
                    Ok(())
                }
            }
            // Transport failures and everything else: outcome unknown.
            _ => Ok(()),
        }
    }
}

/// Safe-direction resolution of an ambiguous `pending` marker: a signed,
/// authenticated read-back that finds the evidence promotes it to `accepted`;
/// anything else — including a failed or empty read — leaves it pending.
/// Absence is not proof the prior write failed, so this can only ever move a
/// key toward `accepted`, never silently re-enable a post.
async fn pending_readback_confirms(client: &BuzzClient, channel: &str, sources: &[String]) -> bool {
    // The distiller records source-event provenance; any graph term containing
    // one of our source ids is a positive confirmation.
    let Some(first) = sources.first() else {
        return false;
    };
    let needle = first.to_ascii_lowercase();
    let sparql = format!(
        "ASK {{ ?s ?p ?o . FILTER(CONTAINS(LCASE(STR(?o)), \"{needle}\") || CONTAINS(LCASE(STR(?s)), \"{needle}\")) }}"
    );
    let request = serde_json::json!({
        "channelId": channel,
        "operation": "semantic_query",
        "scope": { "type": "current_channel" },
        "arguments": { "sparql": sparql, "view": "both" }
    });
    match client.post_authed_json("/api/dkg/query", &request).await {
        Ok(response) => {
            serde_json::from_str::<serde_json::Value>(&response)
                .ok()
                .and_then(|value| {
                    value
                        .get("boolean")
                        .or_else(|| value.get("result").and_then(|r| r.get("boolean")))
                        .and_then(serde_json::Value::as_bool)
                })
                == Some(true)
        }
        Err(_) => false,
    }
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
    // `--force` is a human judgement about an ambiguous ledger; an unattended
    // scheduler must never wield it (agent-panel decision).
    if force {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            return Err(CliError::Usage(
                "--force requires an interactive terminal; schedulers must never use it".into(),
            ));
        }
    }
    let mut ledger = match dedupe_state {
        Some(path) => {
            let (ledger, gate) = ProposalLedger::open(path, channel, sources, force)?;
            match gate {
                LedgerGate::Proceed => Some(ledger),
                LedgerGate::Skip => {
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
                LedgerGate::PendingAmbiguous => {
                    // Safe-direction self-resolution: promote only on a
                    // positive, authenticated read-back.
                    if pending_readback_confirms(client, channel, sources).await {
                        let mut ledger = ledger;
                        ledger.record_accepted()?;
                        println!(
                            "{}",
                            serde_json::json!({
                                "status": "skipped",
                                "reason": "pending marker verified against the relay and promoted to accepted",
                                "channel": channel,
                                "sources": sources,
                            })
                        );
                        return Ok(());
                    }
                    return Err(CliError::Other(format!(
                        "a previous run posted this evidence set without recording the outcome, \
                         and a read-back could not confirm it landed; the relay may already hold \
                         it. Verify the channel's memory, then re-run with --force (interactive \
                         only) or clear the pending key in {path}"
                    )));
                }
            }
        }
        None => None,
    };
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
    if let Some(ledger) = ledger.as_mut() {
        ledger.mark_pending()?;
    }
    let response = match client.post_authed_json("/api/dkg/memory", &value).await {
        Ok(response) => response,
        Err(error) => {
            if let Some(ledger) = ledger.as_mut() {
                ledger.record_failure(&error)?;
            }
            return Err(error);
        }
    };
    if let Some(ledger) = ledger.as_mut() {
        ledger.record_accepted()?;
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
    fn ledger_roundtrips_pending_and_accepted_and_survives_a_missing_file() {
        let dir = std::env::temp_dir().join(format!("buzz-dedupe-{}", std::process::id()));
        let path = dir.join("state.json");
        let _ = std::fs::remove_dir_all(&dir);

        // A first run has no ledger yet; that is not an error.
        let empty = read_dedupe_state(&path).expect("missing ledger reads empty");
        assert!(empty.accepted.is_empty() && empty.pending.is_empty());

        let mut ledger = DedupeLedger::default();
        ledger.accepted.insert("channel:aaa".to_string());
        ledger.pending.insert("channel:bbb".to_string());
        write_dedupe_state(&path, &ledger).expect("write ledger");

        let read = read_dedupe_state(&path).expect("read ledger");
        assert_eq!(read.accepted, ledger.accepted);
        assert_eq!(
            read.pending, ledger.pending,
            "pending must survive a restart"
        );

        // The temp file is process-unique and must not be left behind.
        assert!(!path.with_extension("tmp").exists());
        let strays: Vec<_> = std::fs::read_dir(&dir)
            .expect("list dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            strays.is_empty(),
            "no temp file may survive a successful write"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_accepted_only_array_is_still_readable() {
        let dir = std::env::temp_dir().join(format!("buzz-dedupe-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("state.json");
        std::fs::write(&path, r#"["channel:aaa"]"#).expect("seed legacy ledger");

        let read = read_dedupe_state(&path).expect("read legacy ledger");
        assert!(read.accepted.contains("channel:aaa"));
        assert!(read.pending.is_empty());

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
    fn lock_is_exclusive_and_released_on_drop() {
        let dir = std::env::temp_dir().join(format!("buzz-dedupe-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("state.json");

        let held = LedgerLock::acquire(&path).expect("first lock");
        // A concurrent scheduler must not proceed to read-then-post.
        assert!(
            LedgerLock::acquire(&path).is_err(),
            "a second proposer must not acquire the lock"
        );
        drop(held);
        // Released, so the next run can proceed.
        let _next = LedgerLock::acquire(&path).expect("lock is reusable after drop");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn behavior_ledger(dir_tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("buzz-propose-{dir_tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        (dir.clone(), dir.join("ledger.json"))
    }

    fn behavior_client(base_url: &str) -> crate::client::BuzzClient {
        crate::client::BuzzClient::new(base_url.to_string(), nostr::Keys::generate(), None, None)
            .expect("test client")
    }

    /// A ledger that already accepted this evidence set must skip BEFORE any
    /// network or stdin work: the mock relay observes zero requests.
    #[tokio::test]
    async fn accepted_ledger_skips_before_any_network_request() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_handler = hits.clone();
        let app = axum::Router::new().fallback(axum::routing::any(move || {
            let hits = hits_handler.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                "unexpected"
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let (dir, ledger_path) = behavior_ledger("skip");
        let channel = "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b";
        let sources = vec!["aa".repeat(32)];
        let mut ledger = DedupeLedger::default();
        ledger.accepted.insert(dedupe_key(channel, &sources));
        write_dedupe_state(&ledger_path, &ledger).expect("seed ledger");

        let client = behavior_client(&format!("http://{addr}"));
        let result = propose(
            &client,
            channel,
            &sources,
            "/nonexistent/never-read.json", // must not be read on the skip path
            Some(ledger_path.to_str().unwrap()),
            false,
        )
        .await;
        assert!(result.is_ok(), "skip is success: {result:?}");
        assert_eq!(
            hits.load(Ordering::SeqCst),
            0,
            "no request may reach the relay"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A pending marker whose read-back cannot confirm the write must refuse to
    /// post — at-least-once made visible instead of silent duplication.
    #[tokio::test]
    async fn unconfirmed_pending_marker_refuses_to_post() {
        // Read-back query answers "false"; a subsequent memory POST would be a
        // duplicate risk and must never happen.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let memory_hits = Arc::new(AtomicUsize::new(0));
        let memory_handler = memory_hits.clone();
        let app = axum::Router::new()
            .route(
                "/api/dkg/query",
                axum::routing::post(|| async { axum::Json(serde_json::json!({"boolean": false})) }),
            )
            .route(
                "/api/dkg/memory",
                axum::routing::post(move || {
                    let hits = memory_handler.clone();
                    async move {
                        hits.fetch_add(1, Ordering::SeqCst);
                        "stored"
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let (dir, ledger_path) = behavior_ledger("pending");
        let channel = "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b";
        let sources = vec!["bb".repeat(32)];
        let mut ledger = DedupeLedger::default();
        ledger.pending.insert(dedupe_key(channel, &sources));
        write_dedupe_state(&ledger_path, &ledger).expect("seed ledger");

        let client = behavior_client(&format!("http://{addr}"));
        let result = propose(
            &client,
            channel,
            &sources,
            "/nonexistent/never-read.json",
            Some(ledger_path.to_str().unwrap()),
            false,
        )
        .await;
        assert!(result.is_err(), "unconfirmed pending must fail closed");
        assert_eq!(
            memory_hits.load(Ordering::SeqCst),
            0,
            "the ambiguous evidence set must not be posted again"
        );
        // The marker survives for the next run.
        let after = read_dedupe_state(&ledger_path).expect("read ledger");
        assert!(after.pending.contains(&dedupe_key(channel, &sources)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `--force` is a human affordance; in a non-interactive context (as in
    /// this test harness) it must be refused outright.
    #[tokio::test]
    async fn force_is_refused_without_an_interactive_terminal() {
        let client = behavior_client("http://127.0.0.1:9");
        let result = propose(
            &client,
            "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b",
            &["cc".repeat(32)],
            "/nonexistent/never-read.json",
            None,
            true,
        )
        .await;
        match result {
            Err(CliError::Usage(message)) => {
                assert!(message.contains("interactive"), "got: {message}")
            }
            other => panic!("expected a usage refusal, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_conflict_reconciles_pending_to_accepted() {
        let (dir, ledger_path) = behavior_ledger("conflict");
        let channel = "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b";
        let sources = vec!["dd".repeat(32)];
        let (mut ledger, _) =
            ProposalLedger::open(ledger_path.to_str().unwrap(), channel, &sources, false)
                .expect("open ledger");
        ledger.mark_pending().expect("mark pending");
        ledger
            .record_failure(&CliError::Relay {
                status: 409,
                body: "duplicate proposal for this evidence set".into(),
            })
            .expect("classify conflict");
        drop(ledger);
        let after = read_dedupe_state(&ledger_path).expect("read ledger");
        let key = dedupe_key(channel, &sources);
        assert!(
            after.accepted.contains(&key),
            "conflict means the record exists"
        );
        assert!(!after.pending.contains(&key));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unclassified_failure_keeps_the_pending_marker() {
        let (dir, ledger_path) = behavior_ledger("unknown");
        let channel = "0b6b1f1a-2c3d-4e5f-8a9b-0c1d2e3f4a5b";
        let sources = vec!["ee".repeat(32)];
        let (mut ledger, _) =
            ProposalLedger::open(ledger_path.to_str().unwrap(), channel, &sources, false)
                .expect("open ledger");
        ledger.mark_pending().expect("mark pending");
        ledger
            .record_failure(&CliError::Relay {
                status: 503,
                body: "unavailable".into(),
            })
            .expect("classify unknown");
        drop(ledger);
        let after = read_dedupe_state(&ledger_path).expect("read ledger");
        let key = dedupe_key(channel, &sources);
        assert!(
            after.pending.contains(&key),
            "unknown outcome must fail closed"
        );
        assert!(!after.accepted.contains(&key));
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
