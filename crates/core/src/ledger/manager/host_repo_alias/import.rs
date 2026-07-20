//! plan_ref:
//!   - 04_repository#host-repo-alias-contract

use super::HOST_REPO_ALIAS_IMPORT_MAX_BYTES;
use super::membership::LocalRepoAdmission;
use super::model::{
    HostRepoAliasError, HostRepoAliasImportSummary, HostRepoAliasImportWarning,
    HostRepoAliasImportWarningReason, HostRepoAliasValidationError, normalize_alias,
};
use super::store::AliasStore;
use crate::models::RepoId;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const IMPORT_MAX_ENTRIES: usize = 4096;
const IMPORT_MAX_REPO_ID_BYTES: usize = 64;
const IMPORT_MAX_TOTAL_ALIAS_BYTES: usize = 512 * 1024;

pub(super) struct ParsedImport {
    entries: Vec<Value>,
}

impl ParsedImport {
    pub(super) fn parse(input: &[u8]) -> Result<Self, HostRepoAliasError> {
        if input.len() > HOST_REPO_ALIAS_IMPORT_MAX_BYTES {
            return Err(HostRepoAliasError::BudgetExceeded {
                budget: "file bytes",
                actual: input.len(),
                limit: HOST_REPO_ALIAS_IMPORT_MAX_BYTES,
            });
        }
        let value: Value = serde_json::from_slice(input)?;
        let object = value
            .as_object()
            .ok_or(HostRepoAliasError::InvalidDocument(
                "top level must be an object",
            ))?;
        if object.len() != 3
            || !object.contains_key("format")
            || !object.contains_key("version")
            || !object.contains_key("aliases")
        {
            return Err(HostRepoAliasError::InvalidDocument(
                "top level must contain exactly format, version, and aliases",
            ));
        }
        let format = object.get("format").and_then(Value::as_str).ok_or(
            HostRepoAliasError::InvalidDocument("format must be a string"),
        )?;
        if format != super::model::EXPORT_FORMAT {
            return Err(HostRepoAliasError::UnsupportedFormat(format.to_owned()));
        }
        let version = object.get("version").and_then(Value::as_u64).ok_or(
            HostRepoAliasError::InvalidDocument("version must be an unsigned integer"),
        )?;
        if version != u64::from(super::model::EXPORT_VERSION) {
            return Err(HostRepoAliasError::UnsupportedVersion(version));
        }
        let entries = object.get("aliases").and_then(Value::as_array).ok_or(
            HostRepoAliasError::InvalidDocument("aliases must be an array"),
        )?;
        if entries.len() > IMPORT_MAX_ENTRIES {
            return Err(HostRepoAliasError::BudgetExceeded {
                budget: "entry count",
                actual: entries.len(),
                limit: IMPORT_MAX_ENTRIES,
            });
        }

        let mut total_alias_bytes = 0usize;
        for entry in entries {
            let Some(object) = entry.as_object() else {
                continue;
            };
            if let Some(repo_id) = object.get("repo_id").and_then(Value::as_str)
                && repo_id.len() > IMPORT_MAX_REPO_ID_BYTES
            {
                return Err(HostRepoAliasError::BudgetExceeded {
                    budget: "repo_id bytes",
                    actual: repo_id.len(),
                    limit: IMPORT_MAX_REPO_ID_BYTES,
                });
            }
            if let Some(alias) = object.get("alias").and_then(Value::as_str) {
                total_alias_bytes = total_alias_bytes.checked_add(alias.trim().len()).ok_or(
                    HostRepoAliasError::BudgetExceeded {
                        budget: "total alias bytes",
                        actual: usize::MAX,
                        limit: IMPORT_MAX_TOTAL_ALIAS_BYTES,
                    },
                )?;
                if total_alias_bytes > IMPORT_MAX_TOTAL_ALIAS_BYTES {
                    return Err(HostRepoAliasError::BudgetExceeded {
                        budget: "total alias bytes",
                        actual: total_alias_bytes,
                        limit: IMPORT_MAX_TOTAL_ALIAS_BYTES,
                    });
                }
            }
        }
        Ok(Self {
            entries: entries.clone(),
        })
    }
}

pub(super) struct ImportEvaluation {
    pub(super) summary: HostRepoAliasImportSummary,
    accepted: Vec<(RepoId, String)>,
}

impl ImportEvaluation {
    pub(super) fn apply(&self, store: &mut AliasStore) -> Result<bool, HostRepoAliasError> {
        let mut changed = false;
        for (repo_id, alias) in &self.accepted {
            let expected = store.binding_or_fallback(*repo_id).alias_revision;
            changed |= store.set(*repo_id, alias.clone(), expected)?.changed;
        }
        Ok(changed)
    }
}

pub(super) fn evaluate_import<F>(
    parsed: &ParsedImport,
    store: &AliasStore,
    mut is_active_local_repo: F,
) -> Result<ImportEvaluation, HostRepoAliasError>
where
    F: FnMut(RepoId) -> Result<LocalRepoAdmission, HostRepoAliasError>,
{
    let duplicate_ids = duplicate_repo_ids(&parsed.entries);
    let mut accepted = Vec::new();
    let mut warnings = Vec::new();
    let mut changed = 0usize;
    let mut unchanged = 0usize;

    for (index, entry) in parsed.entries.iter().enumerate() {
        let Some(object) = entry.as_object() else {
            warnings.push(warning(
                index,
                None,
                HostRepoAliasImportWarningReason::EntryNotObject,
            ));
            continue;
        };
        let raw_repo_id = match object.get("repo_id") {
            None => {
                warnings.push(warning(
                    index,
                    None,
                    HostRepoAliasImportWarningReason::RepoIdMissing,
                ));
                continue;
            }
            Some(Value::String(value)) => value,
            Some(_) => {
                warnings.push(warning(
                    index,
                    None,
                    HostRepoAliasImportWarningReason::RepoIdNotString,
                ));
                continue;
            }
        };
        let repo_id = match RepoId::parse_str(raw_repo_id) {
            Ok(repo_id) => repo_id,
            Err(_) => {
                warnings.push(warning(
                    index,
                    None,
                    HostRepoAliasImportWarningReason::RepoIdInvalid,
                ));
                continue;
            }
        };
        if duplicate_ids.contains(&repo_id) {
            warnings.push(warning(
                index,
                Some(repo_id),
                HostRepoAliasImportWarningReason::DuplicateRepoId,
            ));
            continue;
        }
        if object.len() != 2 || !object.contains_key("alias") {
            let reason = if object.contains_key("alias") {
                HostRepoAliasImportWarningReason::EntrySchemaInvalid
            } else {
                HostRepoAliasImportWarningReason::AliasMissing
            };
            warnings.push(warning(index, Some(repo_id), reason));
            continue;
        }
        let raw_alias = match object.get("alias") {
            Some(Value::String(value)) => value,
            Some(_) => {
                warnings.push(warning(
                    index,
                    Some(repo_id),
                    HostRepoAliasImportWarningReason::AliasNotString,
                ));
                continue;
            }
            None => unreachable!("object length/schema branch handled missing alias"),
        };
        let alias = match normalize_alias(raw_alias) {
            Ok(alias) => alias,
            Err(error) => {
                warnings.push(warning(
                    index,
                    Some(repo_id),
                    match error {
                        HostRepoAliasValidationError::Empty => {
                            HostRepoAliasImportWarningReason::AliasEmpty
                        }
                        HostRepoAliasValidationError::TooLong => {
                            HostRepoAliasImportWarningReason::AliasTooLong
                        }
                        HostRepoAliasValidationError::ContainsControl => {
                            HostRepoAliasImportWarningReason::AliasContainsControl
                        }
                    },
                ));
                continue;
            }
        };
        match is_active_local_repo(repo_id)? {
            LocalRepoAdmission::Active => {}
            LocalRepoAdmission::Unknown => {
                warnings.push(warning(
                    index,
                    Some(repo_id),
                    HostRepoAliasImportWarningReason::UnknownLocalRepo,
                ));
                continue;
            }
        }
        let current = store.binding_or_fallback(repo_id);
        if current.alias_revision != 0 && current.alias == alias {
            unchanged += 1;
        } else {
            changed += 1;
        }
        accepted.push((repo_id, alias));
    }

    let accepted_count = accepted.len();
    Ok(ImportEvaluation {
        summary: HostRepoAliasImportSummary {
            accepted: accepted_count,
            changed,
            unchanged,
            skipped: warnings.len(),
            warnings,
        },
        accepted,
    })
}

fn duplicate_repo_ids(entries: &[Value]) -> HashSet<RepoId> {
    let mut counts = HashMap::new();
    for entry in entries {
        let Some(raw) = entry
            .as_object()
            .and_then(|object| object.get("repo_id"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Ok(repo_id) = RepoId::parse_str(raw) else {
            continue;
        };
        *counts.entry(repo_id).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(repo_id, count)| (count > 1).then_some(repo_id))
        .collect()
}

fn warning(
    index: usize,
    repo_id: Option<RepoId>,
    reason: HostRepoAliasImportWarningReason,
) -> HostRepoAliasImportWarning {
    HostRepoAliasImportWarning {
        index,
        repo_id,
        reason,
    }
}
