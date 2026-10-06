//! Durable finite-catalog publication. Archived bytes are historical evidence;
//! freshness after restart requires replay under the selected authority.
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use mrr_relation::Value;
use serde::{Deserialize, Serialize};

use super::transformation::{
    TransformationError, TransformationLimits, TransformationPlanCandidate,
    admit_transformation_plan, digest,
};
use super::transformation_execution::{
    TransformationExecutionReceipt, execute_transformation_plan,
};
use super::transformation_finite::{FiniteTransformationCatalog, FiniteTransformationOwner};
use super::truth::TruthStatus;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArchivedResult {
    version: u16,
    plan: [u8; 32],
    request: [u8; 32],
    scope: [u8; 32],
    execution: [u8; 32],
    answer: Value,
    supports: Vec<[u8; 32]>,
    invalidated: bool,
}
/// A process-exclusive archive. OS file locking is released on process death.
/// Open never converts serialized evidence into an opaque admission receipt.
#[derive(Debug)]
pub struct TransformationResultStore {
    path: PathBuf,
    _lock: File,
    archived: Option<ArchivedResult>,
    current: bool,
    authority: Option<FiniteTransformationCatalog>,
    reverse: BTreeMap<[u8; 32], Vec<[u8; 32]>>,
    limits: TransformationLimits,
}
impl TransformationResultStore {
    pub fn open(
        path: impl AsRef<Path>,
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or(TransformationError::Encoding)?;
        fs::create_dir_all(parent).map_err(|_| TransformationError::Encoding)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .map_err(|_| TransformationError::Encoding)?;
        lock.try_lock().map_err(|_| TransformationError::Conflict)?;
        let archived = match File::open(&path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(limits.max_bytes.get() as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| TransformationError::Encoding)?;
                if bytes.len() > limits.max_bytes.get() {
                    return Err(TransformationError::Budget);
                }
                let mut reader = bytes.as_slice();
                let record: ArchivedResult = ciborium::de::from_reader(&mut reader)
                    .map_err(|_| TransformationError::Encoding)?;
                if !reader.is_empty()
                    || record.version != 1
                    || record.supports.len() > limits.max_dependencies.get()
                {
                    return Err(TransformationError::Encoding);
                }
                let mut canonical = Vec::new();
                ciborium::ser::into_writer(&record, &mut canonical)
                    .map_err(|_| TransformationError::Encoding)?;
                if canonical != bytes {
                    return Err(TransformationError::Encoding);
                }
                Some(record)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(TransformationError::Encoding),
        };
        let mut store = Self {
            path,
            _lock: lock,
            archived,
            current: false,
            authority: None,
            reverse: BTreeMap::new(),
            limits,
        };
        store.index();
        Ok(store)
    }
    fn index(&mut self) {
        self.reverse.clear();
        if let Some(record) = &self.archived {
            for support in &record.supports {
                self.reverse.entry(*support).or_default().push(record.plan);
            }
        }
    }
    #[must_use]
    pub fn freshness(&self) -> TruthStatus {
        if self.current
            && self
                .authority
                .as_ref()
                .is_some_and(|authority| authority.is_current())
        {
            TruthStatus::True
        } else if self.archived.is_some() {
            TruthStatus::Stale
        } else {
            TruthStatus::Unknown
        }
    }
    #[must_use]
    pub fn historical_answer(&self) -> Option<&Value> {
        self.archived.as_ref().map(|r| &r.answer)
    }
    #[must_use]
    pub fn affected_plans(&self, support: &[u8; 32]) -> &[[u8; 32]] {
        self.reverse.get(support).map_or(&[], Vec::as_slice)
    }
    fn write(&self, record: &ArchivedResult) -> Result<(), TransformationError> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(record, &mut bytes)
            .map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > self.limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        // The store's exclusive lock owns this temporary path, including crash recovery.
        let pending = self.path.with_extension("pending");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&pending)
            .map_err(|_| TransformationError::Encoding)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| TransformationError::Encoding)?;
        fs::rename(&pending, &self.path).map_err(|_| TransformationError::Encoding)?;
        File::open(self.path.parent().ok_or(TransformationError::Encoding)?)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| TransformationError::PublicationUncertain)
    }
    /// All owner revocations affecting a support invalidate this historical result.
    /// Invalidate catalog authority first, then call this durable reverse index.
    pub fn invalidate(&mut self, support: &[u8; 32]) -> Result<bool, TransformationError> {
        if self.affected_plans(support).is_empty() {
            return Ok(false);
        }
        self.current = false;
        let mut record = self.archived.clone().ok_or(TransformationError::Unknown)?;
        record.invalidated = true;
        self.write(&record)?;
        self.archived = Some(record);
        Ok(true)
    }
    /// Serialize owner revocation and durable reverse invalidation against
    /// every publication sharing this catalog. Restart retains the tombstone.
    pub fn revoke_support(
        &mut self,
        catalog: &impl FiniteTransformationOwner,
        support: &[u8; 32],
    ) -> Result<bool, TransformationError> {
        if self.affected_plans(support).is_empty() {
            return Ok(false);
        }
        catalog.authority().revoke_with(|| self.invalidate(support))
    }
    fn check_slot(
        &self,
        candidate: &TransformationPlanCandidate,
    ) -> Result<(), TransformationError> {
        if self.archived.as_ref().is_some_and(|record| {
            record.request != candidate.binding.request || record.scope != candidate.binding.scope
        }) {
            return Err(TransformationError::BindingMismatch);
        }
        Ok(())
    }
    fn supports(
        &self,
        candidate: &TransformationPlanCandidate,
    ) -> Result<Vec<[u8; 32]>, TransformationError> {
        let mut supports = Vec::new();
        for step in &candidate.steps {
            supports.push(*step.admission.digest());
            supports.extend(&step.admission.definition().dependencies);
            supports.extend(&step.admission.definition().requirements);
            supports.push(digest(&step.admission.id(), self.limits)?);
            for revision in step.admission.evidence().source_revisions.iter() {
                supports.push(digest(revision, self.limits)?);
            }
        }
        supports.push(candidate.solver);
        supports.push(digest(&candidate.binding, self.limits)?);
        supports.sort_unstable();
        supports.dedup();
        if supports.len() > self.limits.max_dependencies.get() {
            return Err(TransformationError::Budget);
        }
        Ok(supports)
    }
    /// Revalidate, execute, and atomically publish while authority is locked.
    /// No serialized admission is accepted; every publication runs the oracle.
    pub fn execute_and_publish(
        &mut self,
        candidate: &TransformationPlanCandidate,
        source_input: Value,
        catalog: &impl FiniteTransformationOwner,
    ) -> Result<TransformationExecutionReceipt, TransformationError> {
        self.check_slot(candidate)?;
        let plan = admit_transformation_plan(candidate, self.limits, catalog)?;
        let execution = execute_transformation_plan(
            candidate,
            &plan,
            source_input,
            self.limits,
            catalog,
            catalog,
        )?;
        let supports = self.supports(candidate)?;
        let record = ArchivedResult {
            version: 1,
            request: candidate.binding.request,
            scope: candidate.binding.scope,
            plan: *plan.digest(),
            execution: *execution.digest(),
            answer: execution.answer().clone(),
            supports,
            invalidated: false,
        };
        catalog.authority().with_current(&candidate.binding, || {
            if let Err(error) = self.write(&record) {
                if error == TransformationError::PublicationUncertain {
                    self.current = false;
                }
                return Err(error);
            }
            self.archived = Some(record);
            self.current = true;
            self.authority = Some(catalog.authority().clone());
            self.index();
            Ok(())
        })?;
        Ok(execution)
    }
    /// Recover freshness only through actual replay and exact receipt equality.
    pub fn replay(
        &mut self,
        candidate: &TransformationPlanCandidate,
        source_input: Value,
        catalog: &impl FiniteTransformationOwner,
    ) -> Result<TransformationExecutionReceipt, TransformationError> {
        let archived = self.archived.as_ref().ok_or(TransformationError::Unknown)?;
        if archived.invalidated {
            return Err(TransformationError::Revoked);
        }
        self.check_slot(candidate)?;
        let plan = admit_transformation_plan(candidate, self.limits, catalog)?;
        let execution = execute_transformation_plan(
            candidate,
            &plan,
            source_input,
            self.limits,
            catalog,
            catalog,
        )?;
        if archived.supports != self.supports(candidate)?
            || archived.plan != *plan.digest()
            || archived.execution != *execution.digest()
            || archived.answer != *execution.answer()
        {
            return Err(TransformationError::BindingMismatch);
        }
        catalog.authority().with_current(&candidate.binding, || {
            self.current = true;
            self.authority = Some(catalog.authority().clone());
            Ok(())
        })?;
        Ok(execution)
    }
}
