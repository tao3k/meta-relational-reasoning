use super::{
    FiniteProblem, FiniteTransformationCatalog, FiniteTransport, Sha256, TransformationError,
    TransformationResultStore, TransformationStep, TruthStatus, Value, ValueSchema,
    admit_transformation, admit_transformation_plan, binding, execute_transformation_plan,
    finite_catalog, finite_plan, limits, transformation_value_digest,
};
use sha2::Digest;

#[test]
fn finite_native_exhaustive_soundness_and_rejection() {
    let (catalog, candidate) = finite_catalog();
    let plan = admit_transformation_plan(&candidate, limits(), &catalog).unwrap();
    let execution = execute_transformation_plan(
        &candidate,
        &plan,
        Value::Integer(1),
        limits(),
        &catalog,
        &catalog,
    )
    .unwrap();
    assert_eq!(execution.answer(), &Value::Integer(1));
    let p = FiniteProblem {
        domain: [83; 32],
        answers: 2,
        correct: vec![vec![0, 1], vec![1]],
    };
    let mut transport = FiniteTransport {
        lineage: vec![],
        source: FiniteProblem {
            domain: [84; 32],
            answers: 2,
            correct: vec![vec![0], vec![1]],
        },
        target: p,
        forward: vec![0, 1],
        extract: vec![vec![0, 0], vec![0, 1]],
    };
    assert!(
        FiniteTransformationCatalog::new(vec![transport.clone()], binding(1), limits()).is_ok()
    );
    // Break a non-primary correct witness, which a single-answer spot check misses.
    transport.extract[0][1] = 1;
    assert_eq!(
        FiniteTransformationCatalog::new(vec![transport], binding(1), limits()).unwrap_err(),
        TransformationError::Rejected
    );
    let mut altered = candidate.clone();
    altered.steps[0].parameters = [99; 32];
    assert_eq!(
        admit_transformation_plan(&altered, limits(), &catalog).unwrap_err(),
        TransformationError::InstanceMismatch
    );
    altered = candidate.clone();
    altered.solver = [99; 32];
    assert_eq!(
        admit_transformation_plan(&altered, limits(), &catalog).unwrap_err(),
        TransformationError::Rejected
    );
    catalog.revoke().unwrap();
    assert_eq!(
        admit_transformation_plan(&candidate, limits(), &catalog).unwrap_err(),
        TransformationError::Revoked
    );
}
#[test]
fn finite_durable_publication_replay_locking_and_reverse_invalidation() {
    let (catalog, candidate) = finite_catalog();
    let path = std::env::temp_dir().join(format!("mrr-finite-store-{}.cbor", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(
        TransformationResultStore::open(&path, limits()).unwrap_err(),
        TransformationError::Conflict
    );
    store
        .execute_and_publish(&candidate, Value::Integer(1), &catalog)
        .unwrap();
    assert_eq!(store.freshness(), TruthStatus::True);
    let support = *candidate.steps[0].admission.digest();
    assert_eq!(store.affected_plans(&support).len(), 1);
    let mut altered = candidate.clone();
    altered.solver = [99; 32];
    assert!(
        store
            .execute_and_publish(&altered, Value::Integer(1), &catalog)
            .is_err()
    );
    assert_eq!(store.historical_answer(), Some(&Value::Integer(1)));
    drop(store);
    let mut store = TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(store.freshness(), TruthStatus::Stale);
    store
        .replay(&candidate, Value::Integer(1), &catalog)
        .unwrap();
    assert_eq!(store.freshness(), TruthStatus::True);
    catalog.revoke().unwrap();
    // Freshness checks authority even before reverse-index persistence runs.
    assert_eq!(store.freshness(), TruthStatus::Stale);
    assert!(store.revoke_support(&catalog, &support).unwrap());
    drop(store);
    let mut store = TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(
        store
            .replay(&candidate, Value::Integer(1), &catalog)
            .unwrap_err(),
        TransformationError::Revoked
    );
    drop(store);
    std::fs::write(&path, [0xff]).unwrap();
    assert_eq!(
        TransformationResultStore::open(&path, limits()).unwrap_err(),
        TransformationError::Encoding
    );
    std::fs::remove_file(&path).unwrap();
    let _ = std::fs::remove_file(path.with_extension("lock"));
}
#[test]
fn finite_uci_iris_two_edge_transport_matches_external_labels() {
    let bytes = include_bytes!("../../fixtures/iris.data");
    let text = std::str::from_utf8(bytes).unwrap();
    let labels: Vec<usize> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| match line.rsplit(',').next().unwrap() {
            "Iris-setosa" => 0,
            "Iris-versicolor" => 1,
            "Iris-virginica" => 2,
            label => panic!("unknown class {label}"),
        })
        .collect();
    assert_eq!(labels.len(), 150);
    let domain: [u8; 32] = Sha256::digest(bytes).into();
    assert_eq!(domain, IRIS_SHA256);
    let original = FiniteProblem {
        domain,
        answers: 3,
        correct: labels.iter().map(|label| vec![*label]).collect(),
    };
    let reversed = FiniteProblem {
        domain: Sha256::digest([domain.as_slice(), b"reverse"].concat()).into(),
        answers: 3,
        correct: labels.iter().rev().map(|label| vec![*label]).collect(),
    };
    let renamed = FiniteProblem {
        domain: Sha256::digest([domain.as_slice(), b"reverse-renamed"].concat()).into(),
        answers: 3,
        correct: labels
            .iter()
            .rev()
            .map(|label| vec![(*label + 1) % 3])
            .collect(),
    };
    let first = FiniteTransport {
        lineage: vec![],
        source: original,
        target: reversed.clone(),
        forward: (0..150).rev().collect(),
        extract: vec![vec![0, 1, 2]; 150],
    };
    let second = FiniteTransport {
        lineage: vec![],
        source: reversed,
        target: renamed,
        forward: (0..150).collect(),
        extract: vec![vec![2, 0, 1]; 150],
    };
    let start = std::time::Instant::now();
    let transports = vec![first, second];
    let (_, mut base) = finite_plan(transports.clone(), 0);
    let elan = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("elan"))
        .find(|path| path.is_file())
        .expect("pinned Lean gate requires Elan");
    let mut bounded = limits();
    bounded.max_bytes = std::num::NonZeroUsize::new(65536).unwrap();
    let catalog = crate::KernelCheckedFiniteCatalog::check(
        transports,
        base.binding.clone(),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    for (i, step) in base.steps.iter_mut().enumerate() {
        step.admission = admit_transformation(
            step.admission.definition(),
            &catalog.evidence(i).unwrap(),
            &base.binding,
            bounded,
            &catalog,
        )
        .unwrap();
    }
    assert_eq!(catalog.native().extraction_cells(), 900);
    let composite = catalog.compose(0, 1, &elan, &std::env::temp_dir()).unwrap();
    let native_composite = catalog.native().compose(0, 1).unwrap();
    assert_eq!(
        composite.native().definitions(),
        native_composite.definitions()
    );
    let dependencies = &composite.native().definitions()[0].dependencies;
    assert_eq!(dependencies.len(), 2);
    for definition in catalog.native().definitions() {
        assert!(
            dependencies.contains(
                crate::transformation_identity(definition, bounded)
                    .unwrap()
                    .digest_bytes()
            )
        );
    }

    for (input, expected) in labels.iter().enumerate() {
        let mut candidate = base.clone();
        let source = transformation_value_digest(
            &ValueSchema::Integer,
            &Value::Integer(input as i64),
            limits(),
        )
        .unwrap();
        let target = transformation_value_digest(
            &ValueSchema::Integer,
            &Value::Integer((149 - input) as i64),
            limits(),
        )
        .unwrap();
        candidate.input = source;
        candidate.steps[0].input = source;
        candidate.steps[0].target_input = target;
        candidate.steps[1].input = target;
        candidate.steps[1].target_input = target;
        let plan = admit_transformation_plan(&candidate, bounded, &catalog).unwrap();
        let execution = execute_transformation_plan(
            &candidate,
            &plan,
            Value::Integer(input as i64),
            bounded,
            &catalog,
            &catalog,
        )
        .unwrap();
        let definition = &composite.native().definitions()[0];
        let mut composed = candidate.clone();
        composed.steps = vec![TransformationStep {
            admission: admit_transformation(
                definition,
                &composite.evidence(0).unwrap(),
                &composed.binding,
                bounded,
                &composite,
            )
            .unwrap(),
            input: source,
            target_input: target,
            parameters: composite.native().parameters().unwrap(),
        }];
        composed.solver = composite.native().solver(&composed.target).unwrap();
        let admitted = admit_transformation_plan(&composed, bounded, &composite).unwrap();
        let composed_execution = execute_transformation_plan(
            &composed,
            &admitted,
            Value::Integer(input as i64),
            bounded,
            &composite,
            &composite,
        )
        .unwrap();
        assert_eq!(composed_execution.answer(), execution.answer());
        // The independent expected answer comes straight from the unmodified CSV.
        assert_eq!(execution.answer(), &Value::Integer(*expected as i64));
    }
    catalog.native().revoke().unwrap();
    assert_eq!(
        composite.native().compose(0, 0).unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(
        admit_transformation(
            &composite.native().definitions()[0],
            &composite.evidence(0).unwrap(),
            &base.binding,
            bounded,
            &composite
        )
        .unwrap_err(),
        TransformationError::Revoked
    );
    eprintln!(
        "IRIS-OK rows=150 edges=2 extraction_cells=900 elapsed_ms={}",
        start.elapsed().as_millis()
    );
}

const IRIS_SHA256: [u8; 32] = [
    111, 96, 139, 113, 167, 49, 114, 22, 49, 155, 77, 39, 180, 217, 188, 132, 230, 171, 215, 52,
    237, 167, 135, 43, 113, 164, 88, 86, 158, 38, 86, 192,
];

#[test]
fn finite_catalog_wire_and_resource_bounds() {
    let (catalog, _) = finite_catalog();
    let bytes = catalog.to_bytes().unwrap();
    let restored = FiniteTransformationCatalog::from_bytes(&bytes, binding(1), limits()).unwrap();
    assert_eq!(restored.definitions(), catalog.definitions());
    assert_eq!(restored.extraction_cells(), 9);
    let mut extended = bytes.clone();
    extended.push(0);
    assert_eq!(
        FiniteTransformationCatalog::from_bytes(&extended, binding(1), limits()).unwrap_err(),
        TransformationError::Encoding
    );
    let mut bounded = limits();
    bounded.max_bytes = std::num::NonZeroUsize::new(8).unwrap();
    assert_eq!(
        FiniteTransformationCatalog::from_bytes(&bytes, binding(1), bounded).unwrap_err(),
        TransformationError::Budget
    );
}

#[test]
fn finite_persistent_scope_rejection_preserves_archive() {
    let (catalog, candidate) = finite_catalog();
    let path = std::env::temp_dir().join(format!("mrr-finite-scope-{}.cbor", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = TransformationResultStore::open(&path, limits()).unwrap();
    store
        .execute_and_publish(&candidate, Value::Integer(1), &catalog)
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut what_if = candidate.clone();
    what_if.binding.scope = [99; 32];
    assert_eq!(
        store
            .execute_and_publish(&what_if, Value::Integer(1), &catalog)
            .unwrap_err(),
        TransformationError::BindingMismatch
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(store.freshness(), TruthStatus::True);
    drop(store);
    std::fs::remove_file(&path).unwrap();
    let _ = std::fs::remove_file(path.with_extension("lock"));
}

#[test]
fn finite_actual_kernel_certificate_drives_native_execution_and_publication() {
    use crate::KernelCheckedFiniteCatalog;
    let (native, mut candidate) = finite_catalog();
    let transports: Vec<FiniteTransport> =
        ciborium::de::from_reader(native.to_bytes().unwrap().as_slice()).unwrap();
    let elan = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("elan"))
        .find(|path| path.is_file())
        .expect("pinned Lean gate requires Elan");
    let mut bounded = limits();
    bounded.max_bytes = std::num::NonZeroUsize::new(65536).unwrap();
    let checked = KernelCheckedFiniteCatalog::check(
        transports,
        candidate.binding.clone(),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    assert_ne!(checked.certificate().source_digest(), &[0; 32]);
    for (i, step) in candidate.steps.iter_mut().enumerate() {
        step.admission = admit_transformation(
            step.admission.definition(),
            &checked.evidence(i).unwrap(),
            &candidate.binding,
            bounded,
            &checked,
        )
        .unwrap();
    }
    let path = std::env::temp_dir().join(format!(
        "mrr-finite-kernel-store-{}.cbor",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut store = TransformationResultStore::open(&path, bounded).unwrap();
    let result = store
        .execute_and_publish(&candidate, Value::Integer(1), &checked)
        .unwrap();
    assert_eq!(result.answer(), &Value::Integer(1));
    assert_eq!(store.freshness(), TruthStatus::True);
    let mut altered = checked.evidence(0).unwrap();
    altered.environment = [99; 32];
    assert_eq!(
        admit_transformation(
            &checked.native().definitions()[0],
            &altered,
            &candidate.binding,
            bounded,
            &checked
        )
        .unwrap_err(),
        TransformationError::EvidenceMismatch
    );
    drop(store);
    std::fs::remove_file(&path).unwrap();
    let _ = std::fs::remove_file(path.with_extension("lock"));
}
