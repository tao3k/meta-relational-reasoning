#[cfg(feature = "native-inference")]
use crate::{ClosureStatus, search_transformation_routes};
use crate::{
    EntityId, ExternalRevisionIdentity, GenerationId, RevisionBinding, SemanticSnapshot,
    TransformationAdmission, TransformationBinding, TransformationDefinition,
    TransformationEndpoint, TransformationError, TransformationEvidence, TransformationId,
    TransformationLimits, TransformationPlanCandidate, TransformationProfile,
    TransformationResultSlot, TransformationRuntime, TransformationStep, TransformationVerifier,
    TruthStatus, Value, ValueSchema, admit_transformation, admit_transformation_plan,
    decode_transformation_definition, encode_transformation_definition,
    execute_transformation_plan, transformation_failure_truth, transformation_identity,
    transformation_value_digest,
};
use std::num::NonZeroUsize;

pub(super) fn limits() -> TransformationLimits {
    TransformationLimits {
        max_bytes: NonZeroUsize::new(16384).unwrap(),
        max_schema_nodes: NonZeroUsize::new(32).unwrap(),
        max_schema_depth: NonZeroUsize::new(8).unwrap(),
        max_dependencies: NonZeroUsize::new(16).unwrap(),
        max_steps: NonZeroUsize::new(4).unwrap(),
    }
}
fn endpoint(id: u8) -> TransformationEndpoint {
    TransformationEndpoint {
        semantics: [id; 32],
        input: ValueSchema::Integer,
        result: ValueSchema::Integer,
    }
}
fn definition(a: u8, b: u8) -> TransformationDefinition {
    TransformationDefinition {
        version: 1,
        profile: TransformationProfile::TotalWitnessTransport,
        source: endpoint(a),
        target: endpoint(b),
        forward_artifact: [3; 32],
        extract_artifact: [4; 32],
        codec: [5; 32],
        parameter_contract: [6; 32],
        statement: [7; 32],
        dependencies: vec![[8; 32], [9; 32]],
        requirements: vec![[10; 32]],
    }
}
pub(super) fn binding(generation: u8) -> TransformationBinding {
    let generation = GenerationId::from_canonical_bytes([generation]).unwrap();
    let revision = RevisionBinding::admit(
        ExternalRevisionIdentity::new("fixture", "source", "v1").unwrap(),
        generation,
    )
    .unwrap();
    let snapshot = SemanticSnapshot::admit(generation, vec![revision]).unwrap();
    TransformationBinding::new(&snapshot, [20; 32], [21; 32], [22; 32], 100, 200).unwrap()
}
fn evidence(d: &TransformationDefinition, b: &TransformationBinding) -> TransformationEvidence {
    TransformationEvidence {
        transformation: transformation_identity(d, limits()).unwrap(),
        statement: d.statement,
        forward_artifact: d.forward_artifact,
        extract_artifact: d.extract_artifact,
        codec: d.codec,
        checker: [30; 32],
        toolchain: [31; 32],
        environment: [32; 32],
        assumptions: [33; 32],
        source_revisions: b.revisions().to_vec(),
    }
}
/// Test double only: no semantic certification is claimed for this checker.
struct FixtureVerifier {
    failure: Option<TransformationError>,
    policy: u8,
}
impl TransformationVerifier for FixtureVerifier {
    fn policy(&self) -> [u8; 32] {
        [self.policy; 32]
    }
    fn check_definition(
        &self,
        _: &TransformationDefinition,
        _: &TransformationEvidence,
        _: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.failure.clone().map_or(Ok([40; 32]), Err)
    }
    fn check_step(
        &self,
        _: &TransformationStep,
        _: &TransformationBinding,
        _: usize,
        _: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        self.failure.clone().map_or(Ok([41; 32]), Err)
    }
    fn check_solver(
        &self,
        _: &TransformationEndpoint,
        _: &[u8; 32],
        _: &[u8; 32],
        _: &TransformationBinding,
        _: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        self.failure.clone().map_or(Ok([42; 32]), Err)
    }
}
fn verifier() -> FixtureVerifier {
    FixtureVerifier {
        failure: None,
        policy: 50,
    }
}
fn admitted(a: u8, b: u8, binding: &TransformationBinding) -> TransformationAdmission {
    let d = definition(a, b);
    admit_transformation(&d, &evidence(&d, binding), binding, limits(), &verifier()).unwrap()
}
fn plan() -> TransformationPlanCandidate {
    let binding = binding(1);
    let steps = vec![
        TransformationStep {
            admission: admitted(1, 2, &binding),
            input: [60; 32],
            target_input: [61; 32],
            parameters: [62; 32],
        },
        TransformationStep {
            admission: admitted(2, 3, &binding),
            input: [61; 32],
            target_input: [63; 32],
            parameters: [64; 32],
        },
    ];
    TransformationPlanCandidate {
        binding,
        source: endpoint(1),
        target: endpoint(3),
        input: [60; 32],
        steps,
        solver: [65; 32],
    }
}
#[test]
fn transformation_identity_is_stable_and_domain_separated() {
    let d = definition(1, 2);
    let id = transformation_identity(&d, limits()).unwrap();
    let mut reordered = d.clone();
    reordered.dependencies.reverse();
    assert_eq!(id, transformation_identity(&reordered, limits()).unwrap());
    assert_ne!(
        id.digest_bytes(),
        EntityId::from_canonical_bytes(encode_transformation_definition(&d, limits()).unwrap())
            .unwrap()
            .digest_bytes()
    );
    let b1 = binding(1);
    let b2 = binding(2);
    let a1 = admitted(1, 2, &b1);
    let a2 = admitted(1, 2, &b2);
    assert_eq!(a1.id(), a2.id());
    assert_ne!(a1.digest(), a2.digest());
    let text = id.to_string();
    assert_eq!(text.parse::<TransformationId>().unwrap(), id);
    assert!(text.parse::<EntityId>().is_err());
}
#[test]
fn transformation_content_changes_identity() {
    let d = definition(1, 2);
    let id = transformation_identity(&d, limits()).unwrap();
    let mut changed = d.clone();
    changed.extract_artifact = [99; 32];
    assert_ne!(id, transformation_identity(&changed, limits()).unwrap());
    changed = d;
    changed.target.result = ValueSchema::String;
    assert_ne!(id, transformation_identity(&changed, limits()).unwrap());
}
#[test]
fn transformation_rejects_tampered_evidence_and_source() {
    let d = definition(1, 2);
    let b = binding(1);
    let e = evidence(&d, &b);
    for field in 0..5 {
        let mut tampered = e.clone();
        match field {
            0 => tampered.statement = [99; 32],
            1 => tampered.forward_artifact = [99; 32],
            2 => tampered.extract_artifact = [99; 32],
            3 => tampered.codec = [99; 32],
            _ => {
                tampered.transformation =
                    transformation_identity(&definition(2, 3), limits()).unwrap()
            }
        }
        assert_eq!(
            admit_transformation(&d, &tampered, &b, limits(), &verifier()).unwrap_err(),
            TransformationError::EvidenceMismatch
        );
    }
    let mut stale = e.clone();
    stale.source_revisions.clear();
    assert_eq!(
        admit_transformation(&d, &stale, &b, limits(), &verifier()).unwrap_err(),
        TransformationError::SourceMismatch
    );
    stale = e;
    stale.source_revisions.push(stale.source_revisions[0]);
    assert_eq!(
        admit_transformation(&d, &stale, &b, limits(), &verifier()).unwrap_err(),
        TransformationError::DuplicateDependency
    );
}
#[test]
fn transformation_rejects_duplicate_invalid_and_oversized_definitions() {
    let mut d = definition(1, 2);
    d.dependencies.push(d.dependencies[0]);
    assert_eq!(
        transformation_identity(&d, limits()).unwrap_err(),
        TransformationError::DuplicateDependency
    );
    d = definition(1, 2);
    d.version = 2;
    assert_eq!(
        transformation_identity(&d, limits()).unwrap_err(),
        TransformationError::Version
    );
    d = definition(1, 2);
    d.source.result = ValueSchema::Decimal {
        precision: 0,
        scale: 0,
    };
    assert_eq!(
        transformation_identity(&d, limits()).unwrap_err(),
        TransformationError::InvalidSchema
    );
    d = definition(1, 2);
    d.statement = [0; 32];
    assert_eq!(
        transformation_identity(&d, limits()).unwrap_err(),
        TransformationError::InvalidDigest
    );
    let mut small = limits();
    small.max_bytes = NonZeroUsize::new(10).unwrap();
    assert_eq!(
        transformation_identity(&definition(1, 2), small).unwrap_err(),
        TransformationError::Budget
    );
    d = definition(1, 2);
    for _ in 0..9 {
        d.source.input = ValueSchema::List {
            element: Box::new(d.source.input),
            element_nullable: false,
        };
    }
    assert_eq!(
        transformation_identity(&d, limits()).unwrap_err(),
        TransformationError::Budget
    );
}
#[test]
fn transformation_two_edge_plan_admits_ordered_receipts() {
    let p = plan();
    let receipt = admit_transformation_plan(&p, limits(), &verifier()).unwrap();
    assert_eq!(
        receipt.edges(),
        &[p.steps[0].admission.id(), p.steps[1].admission.id()]
    );
    assert_eq!(receipt.step_receipts().len(), 2);
    assert_eq!(receipt.binding(), &p.binding);
    let mut changed = p.clone();
    changed.steps[0].parameters = [90; 32];
    assert_ne!(
        receipt.digest(),
        admit_transformation_plan(&changed, limits(), &verifier())
            .unwrap()
            .digest()
    );
    changed = p;
    changed.binding.knowledge_time += 1;
    assert_eq!(
        admit_transformation_plan(&changed, limits(), &verifier()).unwrap_err(),
        TransformationError::BindingMismatch
    );
}
#[test]
fn transformation_plan_rejects_disconnected_schema_instance_context_and_policy() {
    let mut p = plan();
    p.steps[1].admission = admitted(4, 3, &p.binding);
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::EndpointMismatch
    );
    p = plan();
    p.steps[1].input = [90; 32];
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::InstanceMismatch
    );
    p = plan();
    p.binding.context = [90; 32];
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::BindingMismatch
    );
    p = plan();
    p.steps[1].admission = admitted(2, 3, &binding(2));
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::BindingMismatch
    );
    assert_eq!(
        admit_transformation_plan(
            &plan(),
            limits(),
            &FixtureVerifier {
                failure: None,
                policy: 51
            }
        )
        .unwrap_err(),
        TransformationError::BindingMismatch
    );
    p = plan();
    p.target.result = ValueSchema::String;
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::EndpointMismatch
    );
}
#[test]
fn transformation_checker_failures_never_emit_admission() {
    for failure in [
        TransformationError::Unknown,
        TransformationError::Conflict,
        TransformationError::Revoked,
        TransformationError::Rejected,
        TransformationError::Budget,
    ] {
        let v = FixtureVerifier {
            failure: Some(failure.clone()),
            policy: 50,
        };
        let d = definition(1, 2);
        let b = binding(1);
        assert_eq!(
            admit_transformation(&d, &evidence(&d, &b), &b, limits(), &v).unwrap_err(),
            failure
        );
        assert_eq!(
            admit_transformation_plan(&plan(), limits(), &v).unwrap_err(),
            failure
        );
    }
}
#[test]
fn transformation_plan_is_bounded_and_requires_solver_and_explicit_identity() {
    let p = plan();
    let mut small = limits();
    small.max_steps = NonZeroUsize::new(1).unwrap();
    assert_eq!(
        admit_transformation_plan(&p, small, &verifier()).unwrap_err(),
        TransformationError::Budget
    );
    let mut p = p;
    p.solver = [0; 32];
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::InvalidDigest
    );
    p.steps.clear();
    assert_eq!(
        admit_transformation_plan(&p, limits(), &verifier()).unwrap_err(),
        TransformationError::EmptyPlan
    );
}

#[cfg(feature = "native-inference")]
fn closure_limits(results: usize) -> mrr_deduction::ClosureLimits {
    mrr_deduction::ClosureLimits::new(
        NonZeroUsize::new(16).unwrap(),
        NonZeroUsize::new(64).unwrap(),
        NonZeroUsize::new(results).unwrap(),
    )
}
#[test]
#[cfg(feature = "native-inference")]
fn transformation_ascent_projects_ordered_routes_without_plan_admission() {
    let p = plan();
    let edges: Vec<_> = p.steps.iter().map(|s| s.admission.clone()).collect();
    let found =
        search_transformation_routes(&edges, &p.binding, limits(), closure_limits(16)).unwrap();
    assert_eq!(found.closure().status(), ClosureStatus::Complete);
    let route = found
        .routes()
        .iter()
        .find(|r| r.source == p.source && r.target == p.target)
        .unwrap();
    assert_eq!(route.edges, vec![*edges[0].digest(), *edges[1].digest()]);
    assert_eq!(found.binding(), &p.binding);
    let partial =
        search_transformation_routes(&edges, &p.binding, limits(), closure_limits(1)).unwrap();
    assert_eq!(partial.closure().status(), ClosureStatus::OutputTruncated);
    assert_eq!(partial.routes().len(), 1);
    let mut revoked = edges;
    revoked[1] = admitted(2, 3, &binding(2));
    assert_eq!(
        search_transformation_routes(&revoked, &p.binding, limits(), closure_limits(16))
            .unwrap_err(),
        TransformationError::BindingMismatch
    );
}
#[test]
#[cfg(feature = "native-inference")]
fn transformation_ascent_handles_cycles_deletions_and_duplicate_edges() {
    let b = binding(1);
    let edges = vec![admitted(1, 2, &b), admitted(2, 3, &b), admitted(3, 1, &b)];
    let found = search_transformation_routes(&edges, &b, limits(), closure_limits(16)).unwrap();
    assert_eq!(found.routes().len(), 9);
    assert!(
        found
            .routes()
            .iter()
            .any(|r| r.source == endpoint(1) && r.target == endpoint(1))
    );
    let after_deletion =
        search_transformation_routes(&edges[..1], &b, limits(), closure_limits(16)).unwrap();
    assert_eq!(after_deletion.routes().len(), 1);
    assert!(
        !after_deletion
            .routes()
            .iter()
            .any(|r| r.source == endpoint(1) && r.target == endpoint(3))
    );
    assert_eq!(
        search_transformation_routes(
            &[edges[0].clone(), edges[0].clone()],
            &b,
            limits(),
            closure_limits(16)
        )
        .unwrap_err(),
        TransformationError::DuplicateDependency
    );
    let mut small = limits();
    small.max_steps = NonZeroUsize::new(1).unwrap();
    assert_eq!(
        search_transformation_routes(&edges, &b, small, closure_limits(16)).unwrap_err(),
        TransformationError::Budget
    );
}

use std::cell::Cell;
struct CurrentVerifier<'a> {
    revoked: &'a Cell<bool>,
    failed_step: Option<usize>,
    solver_missing: bool,
}
impl TransformationVerifier for CurrentVerifier<'_> {
    fn policy(&self) -> [u8; 32] {
        [50; 32]
    }
    fn check_definition(
        &self,
        _: &TransformationDefinition,
        _: &TransformationEvidence,
        _: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        Ok([40; 32])
    }
    fn check_step(
        &self,
        _: &TransformationStep,
        _: &TransformationBinding,
        index: usize,
        _: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        if self.revoked.get() {
            return Err(TransformationError::Revoked);
        }
        if self.failed_step == Some(index) {
            return Err(TransformationError::Unknown);
        }
        Ok([41; 32])
    }
    fn check_solver(
        &self,
        _: &TransformationEndpoint,
        _: &[u8; 32],
        _: &[u8; 32],
        _: &TransformationBinding,
        _: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        if self.solver_missing {
            return Err(TransformationError::Unknown);
        }
        Ok([42; 32])
    }
}
#[test]
#[cfg(feature = "native-inference")]
fn transformation_reachable_route_requires_each_premise_and_solver() {
    let p = plan();
    let revoked = Cell::new(false);
    let edges: Vec<_> = p.steps.iter().map(|s| s.admission.clone()).collect();
    assert!(
        search_transformation_routes(&edges, &p.binding, limits(), closure_limits(16))
            .unwrap()
            .routes()
            .iter()
            .any(|r| r.source == p.source && r.target == p.target)
    );
    for failed_step in [Some(0), Some(1)] {
        let checker = CurrentVerifier {
            revoked: &revoked,
            failed_step,
            solver_missing: false,
        };
        assert_eq!(
            admit_transformation_plan(&p, limits(), &checker).unwrap_err(),
            TransformationError::Unknown
        );
    }
    let checker = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: true,
    };
    assert_eq!(
        admit_transformation_plan(&p, limits(), &checker).unwrap_err(),
        TransformationError::Unknown
    );
}

/// Independently specified bounded fixture: every endpoint answers r = input.
/// Forward uses +1,+2; extraction subtracts the respective amount. Its oracle
/// compares input and answer directly and never calls the extractor.
struct NaturalRuntime<'a> {
    mode: u8,
    revoked: &'a Cell<bool>,
}
fn number(value: &Value) -> Result<i64, TransformationError> {
    match value {
        Value::Integer(n) => Ok(*n),
        _ => Err(TransformationError::InvalidSchema),
    }
}
fn amount(step: &TransformationStep) -> i64 {
    if step.admission.definition().target.semantics == [2; 32] {
        1
    } else {
        2
    }
}
impl TransformationRuntime for NaturalRuntime<'_> {
    fn identity(&self) -> [u8; 32] {
        [80; 32]
    }
    fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        _: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        if self.mode == 4 {
            return Err(TransformationError::Unknown);
        }
        Ok(Value::Integer(
            number(input)? + amount(step) + if self.mode == 1 { 1 } else { 0 },
        ))
    }
    fn solve(
        &self,
        _: &[u8; 32],
        _: &TransformationEndpoint,
        input: &Value,
        _: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        if self.mode == 5 {
            return Ok(Value::String("wrong codec".into()));
        }
        Ok(Value::Integer(
            number(input)? + if self.mode == 2 { 1 } else { 0 },
        ))
    }
    fn extract(
        &self,
        step: &TransformationStep,
        _: &Value,
        answer: &Value,
        _: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        if self.mode == 6 {
            self.revoked.set(true);
        }
        Ok(Value::Integer(
            number(answer)? - amount(step) + if self.mode == 3 { 1 } else { 0 },
        ))
    }
    fn check_answer(
        &self,
        _: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        _: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        if number(input)? != number(answer)? {
            return Err(TransformationError::Rejected);
        }
        Ok([81; 32])
    }
}
fn concrete_plan(input: i64) -> TransformationPlanCandidate {
    let mut p = plan();
    let value_digest = |n| {
        transformation_value_digest(&ValueSchema::Integer, &Value::Integer(n), limits()).unwrap()
    };
    p.input = value_digest(input);
    p.steps[0].input = p.input;
    p.steps[0].target_input = value_digest(input + 1);
    p.steps[1].input = p.steps[0].target_input;
    p.steps[1].target_input = value_digest(input + 3);
    p
}
#[test]
fn transformation_executes_two_edges_against_independent_source_oracle() {
    let revoked = Cell::new(false);
    let checker = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let runtime = NaturalRuntime {
        mode: 0,
        revoked: &revoked,
    };
    for input in 0..32 {
        let p = concrete_plan(input);
        let admitted = admit_transformation_plan(&p, limits(), &checker).unwrap();
        let executed = execute_transformation_plan(
            &p,
            &admitted,
            Value::Integer(input),
            limits(),
            &checker,
            &runtime,
        )
        .unwrap();
        assert_eq!(executed.answer(), &Value::Integer(input));
        assert_eq!(executed.plan_digest(), admitted.digest());
        assert_eq!(executed.answer_checks().len(), 3);
    }
}
#[test]
fn transformation_execution_rejects_bad_forward_answer_extract_codec_unknown_and_late_revocation() {
    for (mode, expected) in [
        (1, TransformationError::InstanceMismatch),
        (2, TransformationError::Rejected),
        (3, TransformationError::Rejected),
        (4, TransformationError::Unknown),
        (5, TransformationError::InvalidSchema),
        (6, TransformationError::Revoked),
    ] {
        let revoked = Cell::new(false);
        let checker = CurrentVerifier {
            revoked: &revoked,
            failed_step: None,
            solver_missing: false,
        };
        let runtime = NaturalRuntime {
            mode,
            revoked: &revoked,
        };
        let p = concrete_plan(4);
        let admitted = admit_transformation_plan(&p, limits(), &checker).unwrap();
        assert_eq!(
            execute_transformation_plan(
                &p,
                &admitted,
                Value::Integer(4),
                limits(),
                &checker,
                &runtime
            )
            .unwrap_err(),
            expected
        );
    }
    let revoked = Cell::new(false);
    let checker = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let p = concrete_plan(4);
    let admitted = admit_transformation_plan(&p, limits(), &checker).unwrap();
    assert_eq!(
        execute_transformation_plan(
            &p,
            &admitted,
            Value::Integer(5),
            limits(),
            &checker,
            &NaturalRuntime {
                mode: 0,
                revoked: &revoked
            }
        )
        .unwrap_err(),
        TransformationError::InstanceMismatch
    );
}

#[test]
fn transformation_wire_revalidates_and_rejects_trailing_unknown_and_duplicate_fields() {
    let d = definition(1, 2);
    let bytes = encode_transformation_definition(&d, limits()).unwrap();
    assert_eq!(
        decode_transformation_definition(&bytes, limits()).unwrap(),
        d
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        decode_transformation_definition(&trailing, limits()).unwrap_err(),
        TransformationError::Encoding
    );
    assert!(decode_transformation_definition(&bytes[..bytes.len() - 1], limits()).is_err());
    let mut wire: ciborium::value::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    if let ciborium::value::Value::Array(frame) = &mut wire
        && let ciborium::value::Value::Map(fields) = &mut frame[1]
    {
        fields.push((
            ciborium::value::Value::Text("unexpected".into()),
            ciborium::value::Value::Bool(true),
        ));
    }
    let mut unknown = Vec::new();
    ciborium::into_writer(&wire, &mut unknown).unwrap();
    assert_eq!(
        decode_transformation_definition(&unknown, limits()).unwrap_err(),
        TransformationError::Encoding
    );
    let mut wire: ciborium::value::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    if let ciborium::value::Value::Array(frame) = &mut wire
        && let ciborium::value::Value::Map(fields) = &mut frame[1]
    {
        fields.push(fields[0].clone());
    }
    let mut duplicate = Vec::new();
    ciborium::into_writer(&wire, &mut duplicate).unwrap();
    assert_eq!(
        decode_transformation_definition(&duplicate, limits()).unwrap_err(),
        TransformationError::Encoding
    );
}

#[test]
fn transformation_publication_retains_stale_result_and_isolates_what_if() {
    let revoked = Cell::new(false);
    let checker = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let runtime = NaturalRuntime {
        mode: 0,
        revoked: &revoked,
    };
    let p = concrete_plan(4);
    let admitted = admit_transformation_plan(&p, limits(), &checker).unwrap();
    let execution = execute_transformation_plan(
        &p,
        &admitted,
        Value::Integer(4),
        limits(),
        &checker,
        &runtime,
    )
    .unwrap();
    let digest = *execution.digest();
    let mut slot = TransformationResultSlot::new(p.binding.clone());
    slot.publish(&p, execution.clone(), limits(), &checker)
        .unwrap();
    assert_eq!(slot.completed().unwrap().freshness(), TruthStatus::True);
    // Unknown or revoked current evidence never replaces completed history.
    slot.invalidate();
    revoked.set(true);
    assert_eq!(
        slot.publish(&p, execution.clone(), limits(), &checker)
            .unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(slot.completed().unwrap().execution().digest(), &digest);
    assert_eq!(slot.completed().unwrap().freshness(), TruthStatus::Stale);
    let mut what_if = p.binding.clone();
    what_if.scope = [91; 32];
    assert_eq!(
        slot.select(what_if.clone()).unwrap_err(),
        TransformationError::BindingMismatch
    );
    let sandbox = TransformationResultSlot::new(what_if);
    assert!(sandbox.completed().is_none());
    // Selecting a different generation stales the previous result and rejects
    // publication of a receipt from the previous cut.
    revoked.set(false);
    slot.select(binding(2)).unwrap();
    assert_eq!(
        slot.publish(&p, execution, limits(), &checker).unwrap_err(),
        TransformationError::BindingMismatch
    );
    assert_eq!(slot.completed().unwrap().execution().digest(), &digest);
    assert_eq!(slot.completed().unwrap().freshness(), TruthStatus::Stale);
}

#[test]
fn transformation_failure_truth_preserves_unknown_incomplete_conflict_and_stale() {
    for (failure, truth) in [
        (TransformationError::Budget, TruthStatus::Incomplete),
        (TransformationError::Conflict, TruthStatus::Conflict),
        (TransformationError::Revoked, TruthStatus::Stale),
        (TransformationError::Unknown, TruthStatus::Unknown),
        (TransformationError::Rejected, TruthStatus::Unknown),
        (TransformationError::EndpointMismatch, TruthStatus::Unknown),
    ] {
        assert_eq!(transformation_failure_truth(&failure), truth);
        assert_ne!(transformation_failure_truth(&failure), TruthStatus::False);
    }
}

#[test]
fn transformation_wire_requires_canonical_sets_and_rejects_nested_extensions() {
    let mut d = definition(1, 2);
    d.dependencies.reverse();
    let mut bytes = Vec::new();
    ciborium::into_writer(&("mrr.transformation.v1", &d), &mut bytes).unwrap();
    assert_eq!(
        decode_transformation_definition(&bytes, limits()).unwrap_err(),
        TransformationError::Encoding
    );
    d.source.input = ValueSchema::Record {
        fields: vec![crate::RelationField::new("n", ValueSchema::Integer, false).unwrap()],
    };
    bytes = encode_transformation_definition(&d, limits()).unwrap();
    let mut raw: ciborium::value::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    fn extend_field(value: &mut ciborium::value::Value) -> bool {
        use ciborium::value::Value as Wire;
        match value {
            Wire::Map(fields)
                if fields
                    .iter()
                    .any(|(key, _)| *key == Wire::Text("name".into())) =>
            {
                fields.push((Wire::Text("ignored-extension".into()), Wire::Bool(true)));
                true
            }
            Wire::Map(fields) => fields.iter_mut().any(|(_, value)| extend_field(value)),
            Wire::Array(values) => values.iter_mut().any(extend_field),
            _ => false,
        }
    }
    assert!(extend_field(&mut raw));
    bytes.clear();
    ciborium::into_writer(&raw, &mut bytes).unwrap();
    assert_eq!(
        decode_transformation_definition(&bytes, limits()).unwrap_err(),
        TransformationError::Encoding
    );
}

#[test]
fn transformation_projection_rejects_forged_paths_and_false_completeness() {
    use crate::{ClosureStatus, TransformationRouteCandidate};
    let p = plan();
    let edges: Vec<_> = p.steps.iter().map(|s| s.admission.clone()).collect();
    let route = TransformationRouteCandidate {
        source: p.source.clone(),
        target: p.target.clone(),
        edges: edges.iter().map(|e| *e.digest()).collect(),
    };
    assert!(
        crate::verify_transformation_projection(
            &edges,
            std::slice::from_ref(&route),
            ClosureStatus::OutputTruncated,
            limits()
        )
        .is_ok()
    );
    assert_eq!(
        crate::verify_transformation_projection(
            &edges,
            std::slice::from_ref(&route),
            ClosureStatus::Complete,
            limits()
        )
        .unwrap_err(),
        TransformationError::Rejected
    );
    let mut forged = route;
    forged.edges.reverse();
    assert_eq!(
        crate::verify_transformation_projection(
            &edges,
            &[forged],
            ClosureStatus::OutputTruncated,
            limits()
        )
        .unwrap_err(),
        TransformationError::EndpointMismatch
    );
}
