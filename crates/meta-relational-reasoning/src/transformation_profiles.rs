//! Concrete finite decision, optimality, and partial-domain adapters.
//! Constructors lower full specifications to exact kernel-qualified witness tables.
use super::{
    transformation::{TransformationBinding, TransformationError, TransformationLimits, digest},
    transformation_finite::{FiniteProblem, FiniteTransport},
    transformation_kernel::KernelCheckedFiniteCatalog,
};
use mrr_relation::Value;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteDecisionProblem {
    pub domain: [u8; 32],
    pub yes: Vec<bool>,
}
impl FiniteDecisionProblem {
    fn lower(&self, limits: TransformationLimits) -> Result<FiniteProblem, TransformationError> {
        if self.domain == [0; 32] || self.yes.is_empty() || self.yes.len() > limits.max_bytes.get()
        {
            return Err(TransformationError::Budget);
        }
        Ok(FiniteProblem {
            domain: digest(&("mrr.finite-decision.v1", self), limits)?,
            answers: 2,
            correct: self.yes.iter().map(|yes| vec![usize::from(*yes)]).collect(),
        })
    }
}
/// Identity boolean extraction forces both positive and negative decisions to agree.
#[derive(Clone, Debug)]
pub struct KernelCheckedDecisionReduction {
    kernel: KernelCheckedFiniteCatalog,
}
impl KernelCheckedDecisionReduction {
    pub fn check(
        source: FiniteDecisionProblem,
        target: FiniteDecisionProblem,
        forward: Vec<usize>,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        let transport = FiniteTransport {
            lineage: vec![],
            source: source.lower(limits)?,
            target: target.lower(limits)?,
            extract: vec![vec![0, 1]; source.yes.len()],
            forward,
        };
        let kernel =
            KernelCheckedFiniteCatalog::check(vec![transport], binding, limits, elan, temporary)?;
        Ok(Self { kernel })
    }
    pub fn kernel(&self) -> &KernelCheckedFiniteCatalog {
        &self.kernel
    }
}

/// Each feasible answer has an owner-declared natural cost. Ties are retained.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteOptimizationProblem {
    pub domain: [u8; 32],
    pub feasible: Vec<Vec<bool>>,
    pub costs: Vec<Vec<u64>>,
}
impl FiniteOptimizationProblem {
    fn lower(&self, limits: TransformationLimits) -> Result<FiniteProblem, TransformationError> {
        let answers = self.costs.first().map_or(0, Vec::len);
        if self.domain == [0; 32] {
            return Err(TransformationError::InvalidDigest);
        }
        if answers == 0
            || self.costs.is_empty()
            || self.costs.len() != self.feasible.len()
            || self
                .costs
                .len()
                .checked_mul(answers)
                .and_then(|n| n.checked_mul(answers))
                .is_none_or(|n| n > limits.max_bytes.get())
        {
            return Err(TransformationError::Budget);
        }
        let mut correct = Vec::with_capacity(self.costs.len());
        for (cost, feasible) in self.costs.iter().zip(&self.feasible) {
            if cost.len() != answers || feasible.len() != answers {
                return Err(TransformationError::Rejected);
            }
            let minimum = cost
                .iter()
                .zip(feasible)
                .filter_map(|(cost, yes)| yes.then_some(*cost))
                .min()
                .ok_or(TransformationError::Unknown)?;
            let row: Vec<_> = cost
                .iter()
                .zip(feasible)
                .enumerate()
                .filter_map(|(answer, (cost, yes))| (*yes && *cost == minimum).then_some(answer))
                .collect();
            // Independent all-candidate predicate qualifies the lowering, including ties.
            for answer in 0..answers {
                let optimal = feasible[answer]
                    && (0..answers).all(|other| !feasible[other] || cost[answer] <= cost[other]);
                if optimal != row.contains(&answer) {
                    return Err(TransformationError::Rejected);
                }
            }
            correct.push(row);
        }
        Ok(FiniteProblem {
            domain: digest(&("mrr.finite-optimization.v1", self), limits)?,
            answers,
            correct,
        })
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteOptimizationTransport {
    pub source: FiniteOptimizationProblem,
    pub target: FiniteOptimizationProblem,
    pub forward: Vec<usize>,
    pub extract: Vec<Vec<usize>>,
}
fn matrix<T: ToString>(rows: &[Vec<T>]) -> String {
    super::transformation_kernel::lookup(
        &rows
            .iter()
            .map(|row| {
                super::transformation_kernel::lookup(
                    &row.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "answer.val",
                    0,
                )
            })
            .collect::<Vec<_>>(),
        "input.val",
        0,
    )
}
fn optimization_contract(
    source: &FiniteOptimizationProblem,
    target: &FiniteOptimizationProblem,
) -> String {
    let mut result = "\nnamespace MRRFiniteProfile\n".to_owned();
    for (name, problem) in [("source", source), ("target", target)] {
        let n = problem.costs.len();
        let m = problem.costs[0].len();
        result.push_str(&format!("def {name}Feasible (input : Fin {n}) (answer : Fin {m}) : Bool := {}\ndef {name}Cost (input : Fin {n}) (answer : Fin {m}) : Nat := {}\ntheorem {name}_optimal : ((List.finRange {n}).all fun input => (List.finRange {m}).all fun answer => MRRFiniteKernel.{name}_0.correct input answer == ({name}Feasible input answer && (List.finRange {m}).all (fun other => !{name}Feasible input other || decide ({name}Cost input answer \u{2264} {name}Cost input other)))) = true := by decide\n#print axioms {name}_optimal\n", matrix(&problem.feasible), matrix(&problem.costs)));
    }
    result.push_str("end MRRFiniteProfile\n");
    result
}
/// Untrusted output rows of an external optimum-specification compiler.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledFiniteOptimizationRows {
    pub source: Vec<Vec<usize>>,
    pub target: Vec<Vec<usize>>,
}
/// This certificate checks transport of every optimum, rather than every feasible witness.
#[derive(Clone, Debug)]
pub struct KernelCheckedOptimizationReduction {
    kernel: KernelCheckedFiniteCatalog,
}
impl KernelCheckedOptimizationReduction {
    pub fn check(
        specification: FiniteOptimizationTransport,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        let rows = CompiledFiniteOptimizationRows {
            source: specification.source.lower(limits)?.correct,
            target: specification.target.lower(limits)?.correct,
        };
        Self::check_compiled(specification, rows, binding, limits, elan, temporary)
    }
    /// Independently kernel-check caller-provided lowered rows against the raw cost specification.
    pub fn check_compiled(
        specification: FiniteOptimizationTransport,
        rows: CompiledFiniteOptimizationRows,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        // Bound and validate before generating matrix source or indexing rows.
        let mut source = specification.source.lower(limits)?;
        let mut target = specification.target.lower(limits)?;
        if rows.source.len() != source.correct.len() || rows.target.len() != target.correct.len() {
            return Err(TransformationError::Rejected);
        }
        source.correct = rows.source;
        target.correct = rows.target;
        let contract = optimization_contract(&specification.source, &specification.target);
        let transport = FiniteTransport {
            lineage: vec![],
            source,
            target,
            forward: specification.forward,
            extract: specification.extract,
        };
        let kernel = KernelCheckedFiniteCatalog::check_with_contract(
            vec![transport],
            binding,
            limits,
            elan,
            temporary,
            &contract,
            &[
                "MRRFiniteProfile.source_optimal",
                "MRRFiniteProfile.target_optimal",
            ],
        )?;
        Ok(Self { kernel })
    }
    pub fn kernel(&self) -> &KernelCheckedFiniteCatalog {
        &self.kernel
    }
}

/// An unavailable input stays unknown. Successful inputs form an explicit compact domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FinitePartialTransport {
    pub source: FiniteProblem,
    pub target: FiniteProblem,
    pub forward: Vec<Option<usize>>,
    pub extract: Vec<Vec<usize>>,
}
#[derive(Clone, Debug)]
pub struct KernelCheckedPartialTransformation {
    kernel: KernelCheckedFiniteCatalog,
    original_inputs: Vec<usize>,
}
impl KernelCheckedPartialTransformation {
    pub fn check(
        partial: FinitePartialTransport,
        binding: TransformationBinding,
        limits: TransformationLimits,
        elan: &Path,
        temporary: &Path,
    ) -> Result<Self, TransformationError> {
        partial.source.validate(limits.max_bytes.get())?;
        partial.target.validate(limits.max_bytes.get())?;
        if partial.forward.len() != partial.source.correct.len()
            || partial.extract.len() != partial.forward.len()
            || partial.forward.len() > limits.max_bytes.get()
        {
            return Err(TransformationError::Rejected);
        }
        let original_inputs: Vec<_> = partial
            .forward
            .iter()
            .enumerate()
            .filter_map(|(i, target)| target.map(|_| i))
            .collect();
        if original_inputs.is_empty() {
            return Err(TransformationError::Unknown);
        }
        // The map is part of the exact endpoint semantics; different restrictions cannot compose.
        let source = FiniteProblem {
            domain: digest(
                &(
                    "mrr.finite-partial-domain.v1",
                    &partial.source,
                    &original_inputs,
                ),
                limits,
            )?,
            answers: partial.source.answers,
            correct: original_inputs
                .iter()
                .map(|i| partial.source.correct[*i].clone())
                .collect(),
        };
        let transport = FiniteTransport {
            lineage: vec![],
            source,
            target: partial.target,
            forward: original_inputs
                .iter()
                .map(|i| partial.forward[*i].expect("successful input"))
                .collect(),
            extract: original_inputs
                .iter()
                .map(|i| partial.extract[*i].clone())
                .collect(),
        };
        let kernel =
            KernelCheckedFiniteCatalog::check(vec![transport], binding, limits, elan, temporary)?;
        Ok(Self {
            kernel,
            original_inputs,
        })
    }
    pub fn kernel(&self) -> &KernelCheckedFiniteCatalog {
        &self.kernel
    }
    /// Required adapter from the original input to the certified restricted instance.
    pub fn source_instance(&self, original: usize) -> Result<Value, TransformationError> {
        let compact = self
            .original_inputs
            .binary_search(&original)
            .map_err(|_| TransformationError::Unknown)?;
        Ok(Value::Integer(
            i64::try_from(compact).map_err(|_| TransformationError::Budget)?,
        ))
    }
    pub fn original_inputs(&self) -> &[usize] {
        &self.original_inputs
    }
}
