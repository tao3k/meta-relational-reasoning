// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
//! Original MRR Context admission checked against POO's registered current Host.
use crate::{
    Error, SemanticRuntime, datum,
    mrr::{MrrBridgeError, MrrFamilyAdmission},
    wire::Value,
};
use meta_relational_reasoning as mrr;
use std::str::FromStr;

/// Retained registration identity, never a permit to use or execute effects.
#[derive(Debug)]
pub struct MrrContextUseGrant {
    identity: String,
    generation: u64,
    admission_digest: Value,
}
fn purpose(p: mrr::AgenticAiContextUsePurpose) -> &'static str {
    match p {
        mrr::AgenticAiContextUsePurpose::Display => "display",
        mrr::AgenticAiContextUsePurpose::Action => "action",
    }
}
fn hex(bytes: &[u8; 32]) -> String {
    format!(
        "sha256:{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}
fn invalid() -> Error {
    Error::Transport("invalid native Context use observation".into())
}
fn text(v: &Value) -> Result<&str, Error> {
    v.as_str().ok_or_else(invalid)
}
fn digest(v: &Value) -> Result<[u8; 32], Error> {
    let s = text(v)?.strip_prefix("sha256:").ok_or_else(invalid)?;
    if s.len() != 64 || !s.is_ascii() {
        return Err(invalid());
    }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| invalid())?;
    }
    Ok(out)
}
fn ids<T: FromStr>(v: &Value) -> Result<Vec<T>, Error> {
    v.as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|x| text(x)?.parse().map_err(|_| invalid()))
        .collect()
}
fn contract(v: &Value) -> Result<mrr::AgenticAiContextContract, Error> {
    if v.as_object().ok_or_else(invalid)?.len() != 6 {
        return Err(invalid());
    }
    Ok(mrr::AgenticAiContextContract {
        actor: text(&v["actor"])?.parse().map_err(|_| invalid())?,
        task: text(&v["task"])?.parse().map_err(|_| invalid())?,
        policy_digest: digest(&v["policyDigest"])?,
        required: ids(&v["required"])?,
        temporal_receipts: ids(&v["temporalReceipts"])?,
        require_complete: match v["requireComplete"] {
            Value::Bool(b) => b,
            _ => return Err(invalid()),
        },
    })
}
fn wire_contract(c: &mrr::AgenticAiContextContract) -> Value {
    datum!({"actor":c.actor.to_string(),"task":c.task.to_string(),"policyDigest":hex(&c.policy_digest),
        "required":c.required.iter().map(|id| Value::from(id.to_string())).collect::<Vec<_>>(),
        "temporalReceipts":c.temporal_receipts.iter().map(|id| Value::from(id.to_string())).collect::<Vec<_>>(),
        "requireComplete":c.require_complete})
}
impl MrrFamilyAdmission {
    /// Trusted Host enrollment binds the original Context to the admitted native family.
    /// Policy registration is produced by the dedicated native policy Host control.
    #[allow(clippy::too_many_arguments)]
    pub fn register_context_use(
        &self,
        runtime: &SemanticRuntime,
        context: &mrr::AdmittedAgenticAiContext,
        bundle: &mrr::ReasoningBundle,
        snapshot: &mrr::SemanticSnapshot,
        identity: &str,
        generation: u64,
        policy_registration: &Value,
        purposes: &[mrr::AgenticAiContextUsePurpose],
    ) -> Result<MrrContextUseGrant, MrrBridgeError> {
        self.check_context_binding(runtime, context, bundle, snapshot)?;
        if policy_registration["schema"] != "poo-flow.temporal-policy-refresh-result.v1"
            || policy_registration["policyIdentity"] != self.result()["conclusion"]["policy"]
        {
            return Err(MrrBridgeError::ContextBindingMismatch);
        }
        let request = datum!({"schema":"poo-flow.context-use-refresh-request.v1",
            "identity":identity,"generation":generation,"manifestDigest":hex(context.manifest().digest()),
            "admissionDigest":&self.result()["admissionDigest"],"contract":wire_contract(&context.manifest().record().contract),
            "policyIdentity":&policy_registration["policyIdentity"],"policyDigest":&policy_registration["policyDigest"],
            "purposes":purposes.iter().map(|p| Value::from(purpose(*p))).collect::<Vec<_>>(),"enabled":true});
        let registered = runtime
            .refresh_context_use(&request)
            .map_err(MrrBridgeError::Native)?;
        if registered["schema"] != "poo-flow.context-use-refresh-result.v1"
            || registered["identity"] != identity
            || registered["generation"] != Value::from(generation)
            || registered["enabled"] != true
        {
            return Err(MrrBridgeError::ContextBindingMismatch);
        }
        Ok(MrrContextUseGrant {
            identity: identity.into(),
            generation,
            admission_digest: self.result()["admissionDigest"].clone(),
        })
    }
    /// Mandatory source, receipt, current contract, revocation and expiry recheck.
    /// A success describes this observation only; it does not authorize later IO.
    #[allow(clippy::too_many_arguments)]
    pub fn check_context(
        &self,
        runtime: &SemanticRuntime,
        context: &mrr::AdmittedAgenticAiContext,
        bundle: &mrr::ReasoningBundle,
        snapshot: &mrr::SemanticSnapshot,
        grant: &MrrContextUseGrant,
        use_purpose: mrr::AgenticAiContextUsePurpose,
    ) -> Result<Value, MrrBridgeError> {
        let mut receipt = self.check_context_binding(runtime, context, bundle, snapshot)?;
        if grant.admission_digest != self.result()["admissionDigest"] {
            return Err(MrrBridgeError::ContextBindingMismatch);
        }
        context
            .check_current_use(
                bundle,
                snapshot,
                use_purpose,
                &NativeAuthority { runtime, grant },
            )
            .map_err(MrrBridgeError::ContextUse)?;
        receipt["schema"] = "poo-flow.mrr-context-use-check.v1".into();
        receipt["purpose"] = purpose(use_purpose).into();
        Ok(receipt)
    }
}
struct NativeAuthority<'a> {
    runtime: &'a SemanticRuntime,
    grant: &'a MrrContextUseGrant,
}
impl mrr::AgenticAiContextUseAuthority for NativeAuthority<'_> {
    type Error = Error;
    fn observe(
        &self,
        _manifest: &mrr::AgenticAiContextManifest,
        p: mrr::AgenticAiContextUsePurpose,
    ) -> Result<mrr::AgenticAiContextUseObservation, Error> {
        let v = self.runtime.call(
            "context.use.observe",
            &datum!({"schema":"poo-flow.context-use-observe-request.v1",
            "identity":&self.grant.identity,"expectedGeneration":self.grant.generation,
            "purpose":purpose(p),"admissionDigest":&self.grant.admission_digest}),
        )?;
        if v["schema"] != "poo-flow.context-use-observation.v1"
            || v["abiVersion"] != 1
            || v["identity"] != self.grant.identity
            || v["purpose"] != purpose(p)
            || v["currentSource"]["schema"] != "poo-flow.temporal-family-applicability.v1"
            || v["currentSource"]["admissionDigest"] != self.grant.admission_digest
            || v["sourceAuthenticated"] != false
            || v["actionAuthorized"] != false
            || v["durable"] != false
        {
            return Err(invalid());
        }
        let decision = match text(&v["decision"])? {
            "allowed" => mrr::AgenticAiContextUseDecision::Allowed,
            "denied" => mrr::AgenticAiContextUseDecision::Denied,
            "revoked" => mrr::AgenticAiContextUseDecision::Revoked,
            "expired" => mrr::AgenticAiContextUseDecision::Expired,
            _ => return Err(invalid()),
        };
        if decision == mrr::AgenticAiContextUseDecision::Allowed
            && (v["currentSource"]["status"] != "current"
                || v["generation"] != Value::from(self.grant.generation))
        {
            return Err(invalid());
        }
        Ok(mrr::AgenticAiContextUseObservation {
            manifest_digest: digest(&v["manifestDigest"])?,
            purpose: p,
            current_contract: contract(&v["contract"])?,
            decision,
        })
    }
}
