import MRR.AgenticAIContext.Closure

namespace MRR.AgenticAIContext

/-- Invalidation uses the union of old and new dependency declarations. -/
def revisionDependencies (oldDeps newDeps : Nat -> List Nat) (id : Nat) : List Nat :=
  oldDeps id ++ newDeps id

def reverseDependencies (domain : List Nat) (deps : Nat -> List Nat) (id : Nat) : List Nat :=
  domain.filter (fun parent => (deps parent).contains id)

/-- Exact semantic values include payload and dependencies. This computational
model takes canonical values; it does not assume a cryptographic digest is injective. -/
def checkRevisionChanges {Value : Type} [DecidableEq Value] (domain changed : List Nat) (oldValue newValue : Nat -> Value)
    (globalChanged : Bool) : Bool :=
  decide (forall id, id inList domain -> (globalChanged = true \/ Not (oldValue id = newValue id)) -> id inList changed)

def revisionReusable (oldSelected newSelected invalidated : List Nat) : List Nat :=
  oldSelected.filter (fun id => newSelected.contains id && !(invalidated.contains id))

/-- Dependency impact propagates backwards over every finite forward path. -/
theorem dependency_impact_ancestor (domain changed : List Nat) (deps : Nat -> List Nat)
    (origin target : Nat) (originSource : origin inList domain)
    (sourceClosed : forall parent, parent inList domain -> forall id, id inList deps parent -> id inList domain)
    (path : Required [origin] deps target)
    (impact : Required changed (reverseDependencies domain deps) target) :
    Required changed (reverseDependencies domain deps) origin := by
  have sourceMember : forall id, Required [origin] deps id -> id inList domain :=
    fun id reached => closed_contains_required [origin] domain deps
      (by
        intro id member
        have equal : id = origin := by simpa using member
        simpa [equal] using originSource) sourceClosed id reached
  induction path with
  | @root id member =>
    have equal : id = origin := by simpa using member
    simpa [equal] using impact
  | @dependency parent id reached edge induction =>
    apply induction
    apply Required.dependency impact
    simpa [reverseDependencies] using And.intro (sourceMember parent reached) edge

/-- A reusable selected fact is present at both endpoints and outside the complete
reverse dependency impact closure. -/
theorem revision_reusable_membership (oldSelected newSelected invalidated : List Nat)
    (id : Nat) (member : id inList revisionReusable oldSelected newSelected invalidated) :
    id inList oldSelected /\ id inList newSelected /\ Not (id inList invalidated) := by
  simpa [revisionReusable] using member

/-- Reuse implies exact equality for the fact and every transitive dependency in
the union graph; changing either an old or new dependency invalidates its users. -/
theorem revision_reuse_dependency_safe {Value : Type} [DecidableEq Value] (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (globalChanged : Bool) (deps : Nat -> List Nat)
    (fuel limit id : Nat)
    (changesOk : checkRevisionChanges domain changed oldValue newValue globalChanged = true)
    (impactOk : checkRequiredClosure domain changed (reverseDependencies domain deps)
      fuel limit invalidated = true)
    (selectedSource : forall selected, selected inList oldSelected -> selected inList domain)
    (sourceClosed : forall parent, parent inList domain -> forall dependency, dependency inList deps parent -> dependency inList domain)
    (member : id inList revisionReusable oldSelected newSelected invalidated) :
    globalChanged = false /\
      forall dependency, Required [id] deps dependency -> oldValue dependency = newValue dependency := by
  have members := revision_reusable_membership oldSelected newSelected invalidated id member
  have originSource := selectedSource id members.1
  have changes := of_decide_eq_true changesOk
  have noImpact : Not (Required changed (reverseDependencies domain deps) id) := by
    intro impact
    exact members.2.2 ((checked_closure_exact domain changed (reverseDependencies domain deps)
      fuel limit invalidated impactOk id).mpr impact)
  have noGlobal : Not (globalChanged = true) := by
    intro global
    exact noImpact (Required.root (changes id originSource (Or.inl global)))
  have globalFalse : globalChanged = false := by cases globalChanged <;> simp_all
  refine And.intro globalFalse ?_
  intro dependency path
  have dependencySource := closed_contains_required [id] domain deps
    (by
      intro root rootMember
      have equal : root = id := by simpa using rootMember
      simpa [equal] using originSource) sourceClosed dependency path
  by_cases equal : oldValue dependency = newValue dependency
  case pos => exact equal
  case neg =>
    have impacted : Required changed (reverseDependencies domain deps) dependency :=
      Required.root (changes dependency dependencySource (Or.inr equal))
    exact False.elim (noImpact (dependency_impact_ancestor domain changed deps id dependency
    originSource sourceClosed path impacted))

/-- Any global query/snapshot/contract drift refuses all semantic reuse. -/
theorem revision_global_change_no_reuse {Value : Type} [DecidableEq Value] (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (deps : Nat -> List Nat) (fuel limit : Nat)
    (changesOk : checkRevisionChanges domain changed oldValue newValue true = true)
    (impactOk : checkRequiredClosure domain changed (reverseDependencies domain deps)
      fuel limit invalidated = true)
    (selectedSource : forall selected, selected inList oldSelected -> selected inList domain) :
    revisionReusable oldSelected newSelected invalidated = [] := by
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro id member
  have members := revision_reusable_membership oldSelected newSelected invalidated id member
  have changes := of_decide_eq_true changesOk
  have changedId := changes id (selectedSource id members.1) (Or.inl rfl)
  exact members.2.2 ((checked_closure_exact domain changed (reverseDependencies domain deps)
    fuel limit invalidated impactOk id).mpr (Required.root changedId))

end MRR.AgenticAIContext

namespace MRR.AgenticAIContext

/-- Protocol publication gate mirrored by the Quint Seal/Publish actions.
This model gate is not yet a source theorem for complete Rust publication. -/
def publishRevision {Value : Type} [DecidableEq Value]
    (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (globalChanged : Bool)
    (deps : Nat -> List Nat) (fuel limit : Nat) : Option (List Nat) :=
  if checkRevisionChanges domain changed oldValue newValue globalChanged then
    if checkRequiredClosure domain changed (reverseDependencies domain deps) fuel limit invalidated then
      some (revisionReusable oldSelected newSelected invalidated)
    else none
  else none

theorem published_revision_gates {Value : Type} [DecidableEq Value]
    (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (globalChanged : Bool)
    (deps : Nat -> List Nat) (fuel limit : Nat) (output : List Nat) :
    publishRevision domain changed oldSelected newSelected invalidated oldValue newValue
      globalChanged deps fuel limit = some output <->
    checkRevisionChanges domain changed oldValue newValue globalChanged = true /\
    checkRequiredClosure domain changed (reverseDependencies domain deps) fuel limit invalidated = true /\
    output = revisionReusable oldSelected newSelected invalidated := by
  by_cases changes : checkRevisionChanges domain changed oldValue newValue globalChanged = true
  case pos =>
    by_cases impact : checkRequiredClosure domain changed (reverseDependencies domain deps)
      fuel limit invalidated = true
    case pos =>
      simp only [publishRevision, changes, impact, ite_true, Option.some.injEq, true_and]
      exact eq_comm
    case neg => simp [publishRevision, changes, impact]
  case neg => simp [publishRevision, changes]

theorem published_revision_dependency_safe {Value : Type} [DecidableEq Value]
    (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (globalChanged : Bool)
    (deps : Nat -> List Nat) (fuel limit id : Nat) (output : List Nat)
    (published : publishRevision domain changed oldSelected newSelected invalidated
      oldValue newValue globalChanged deps fuel limit = some output)
    (selectedSource : forall selected, selected inList oldSelected -> selected inList domain)
    (sourceClosed : forall parent, parent inList domain -> forall dependency,
      dependency inList deps parent -> dependency inList domain)
    (member : id inList output) :
    globalChanged = false /\ forall dependency,
      Required [id] deps dependency -> oldValue dependency = newValue dependency := by
  have gates := (published_revision_gates domain changed oldSelected newSelected invalidated
    oldValue newValue globalChanged deps fuel limit output).mp published
  apply revision_reuse_dependency_safe domain changed oldSelected newSelected invalidated
    oldValue newValue globalChanged deps fuel limit id gates.1 gates.2.1 selectedSource sourceClosed
  simpa [gates.2.2] using member

theorem published_revision_impact_exact {Value : Type} [DecidableEq Value]
    (domain changed oldSelected newSelected invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (globalChanged : Bool)
    (deps : Nat -> List Nat) (fuel limit : Nat) (output : List Nat)
    (published : publishRevision domain changed oldSelected newSelected invalidated
      oldValue newValue globalChanged deps fuel limit = some output) (id : Nat) :
    id inList invalidated <-> Required changed (reverseDependencies domain deps) id := by
  have gates := (published_revision_gates domain changed oldSelected newSelected invalidated
    oldValue newValue globalChanged deps fuel limit output).mp published
  exact checked_closure_exact domain changed (reverseDependencies domain deps)
    fuel limit invalidated gates.2.1 id

end MRR.AgenticAIContext
