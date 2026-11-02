import Forward

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

abbrev NativeImpact := state.ImpactTraversal

def enqueueModel : List NativeFactId -> List NativeFactId -> List NativeFactId ->
    Prod (List NativeFactId) (List NativeFactId)
  | [], scheduled, pending => (scheduled, pending)
  | id :: rest, scheduled, pending =>
    if id inList scheduled then enqueueModel rest scheduled pending
    else enqueueModel rest (id :: scheduled) (pending ++ [id])

/-- Actual iterator loop: each fresh identity is scheduled once and appended in
iterator order. Duplicate neighbors are permitted. The named library iterator
model and the explicit machine-capacity premise remain the trust boundary. -/
theorem native_impact_enqueue_exact (neighbors scheduled : List NativeFactId)
    (pending : alloc.vec.Vec NativeFactId)
    (capacity : pending.val.length + neighbors.length <= Usize.max) :
    exists selected queued,
      state.advance_impact_loop neighbors scheduled pending = .ok (selected, queued) /\
      selected = (enqueueModel neighbors scheduled pending.val).1 /\
      queued.val = (enqueueModel neighbors scheduled pending.val).2 := by
  let expected := enqueueModel neighbors scheduled pending.val
  let invariant := fun input : Prod (List NativeFactId)
      (Prod (List NativeFactId) (alloc.vec.Vec NativeFactId)) =>
    input.2.2.val.length + input.1.length <= Usize.max /\
    enqueueModel input.1 input.2.1 input.2.2.val = expected
  let post := fun output : Prod (List NativeFactId) (alloc.vec.Vec NativeFactId) =>
    output.1 = expected.1 /\ output.2.val = expected.2
  have total : WP.spec (loop
      (fun (iter, selected, queued) => state.advance_impact_loop.body iter selected queued)
      (neighbors, scheduled, pending)) post := by
    apply loop.spec_decr_nat (fun input => input.1.length) invariant post
    next =>
      intro input valid
      cases input
      rename_i iter pair
      cases pair
      rename_i selected queued
      have bound := valid.1
      have exactModel := valid.2
      cases iter with
      | nil =>
        simp only [state.advance_impact_loop.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry]
        apply (WP.spec_ok _).mpr
        exact And.intro (congrArg Prod.fst exactModel) (congrArg Prod.snd exactModel)
      | cons id rest =>
        simp only [state.advance_impact_loop.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry, native_set_insert_exact]
        by_cases known : id inList selected
        case pos =>
          simp only [known, if_true, Bool.false_eq_true, if_false]
          apply (WP.spec_ok _).mpr
          refine And.intro (And.intro ?_ ?_) ?_
          next => change queued.val.length + rest.length <= Usize.max; change queued.val.length + (id :: rest).length <= Usize.max at bound; simp only [List.length_cons] at bound; omega
          next => simpa only [enqueueModel, known, if_true] using exactModel
          next => simp only [List.length_cons]; omega
        case neg =>
          simp only [known, if_false, if_true]
          have pushBound : queued.val.length < Usize.max := by
            change queued.val.length + (id :: rest).length <= Usize.max at bound
            simp only [List.length_cons] at bound; omega
          apply Exists.elim ((WP.spec_equiv_exists _ _).mp (alloc.vec.Vec.push_spec queued id pushBound))
          intro next nextSpec
          simp only [nextSpec.1, bind_ok]
          apply (WP.spec_ok _).mpr
          refine And.intro (And.intro ?_ ?_) ?_
          next =>
            change next.val.length + rest.length <= Usize.max
            rw [nextSpec.2]
            change queued.val.length + (id :: rest).length <= Usize.max at bound
            simp only [List.length_append, List.length_cons, List.length_nil] at *; omega
          next => simpa only [enqueueModel, known, if_false, nextSpec.2] using exactModel
          next => simp only [List.length_cons]; omega
    next => exact And.intro capacity rfl
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro output spec
  exact Exists.intro output.1 (Exists.intro output.2 (And.intro spec.1 spec.2))

/-- Scheduling is duplicate suppression at enqueue time, including duplicates
within the supplied iterator sequence. -/
theorem enqueue_model_members (neighbors scheduled pending : List NativeFactId)
    (id : NativeFactId) :
    id inList (enqueueModel neighbors scheduled pending).1 <->
      id inList scheduled \/ id inList neighbors := by
  induction neighbors generalizing scheduled pending with
  | nil => simp [enqueueModel]
  | cons head rest induction =>
    simp only [enqueueModel]
    by_cases known : head inList scheduled
    case pos =>
      simp only [known, if_true, induction, List.mem_cons]
      constructor
      next => intro member; cases member with
              | inl old => exact Or.inl old
              | inr added => exact Or.inr (Or.inr added)
      next => intro member; cases member with
              | inl old => exact Or.inl old
              | inr added => cases added with
                            | inl equal => exact Or.inl (Eq.mp (congrArg (fun actual => actual inList scheduled) equal.symm) known)
                            | inr restMember => exact Or.inr restMember
    case neg =>
      simp only [known, if_false, induction, List.mem_cons]
      tauto

/-- Each successful fresh insert contributes one pending entry; duplicates
contribute neither. This conservation law supplies the outer-loop rank seam. -/
theorem enqueue_model_length_balance (neighbors scheduled pending : List NativeFactId) :
    (enqueueModel neighbors scheduled pending).1.length + pending.length =
      scheduled.length + (enqueueModel neighbors scheduled pending).2.length := by
  induction neighbors generalizing scheduled pending with
  | nil => rfl
  | cons id rest induction =>
    simp only [enqueueModel]
    by_cases known : id inList scheduled
    case pos => simp only [known, if_true]; exact induction scheduled pending
    case neg =>
      simp only [known, if_false]
      have balance := induction (id :: scheduled) (pending ++ [id])
      simp only [List.length_append, List.length_cons, List.length_nil] at balance
      omega

theorem enqueue_model_selected_nodup (neighbors scheduled pending : List NativeFactId)
    (unique : scheduled.Nodup) : (enqueueModel neighbors scheduled pending).1.Nodup := by
  induction neighbors generalizing scheduled pending with
  | nil => exact unique
  | cons id rest induction =>
    simp only [enqueueModel]
    by_cases known : id inList scheduled
    case pos => simp only [known, if_true]; exact induction scheduled pending unique
    case neg =>
      simp only [known, if_false]
      exact induction (id :: scheduled) (pending ++ [id]) (List.nodup_cons.mpr (And.intro known unique))

def reverseLookup : List (Prod NativeFactId (List NativeFactId)) -> NativeFactId ->
    Option (List NativeFactId)
  | [], _ => none
  | (key, value) :: rest, id => if key = id then some value else reverseLookup rest id

theorem native_reverse_lookup_exact (entries : List (Prod NativeFactId (List NativeFactId)))
    (id : NativeFactId) :
    alloc.collections.btree.map.BTreeMap.get alloc.alloc.Global.Insts.CoreAllocAllocatorClone
      (core.borrow.Borrow.Blanket NativeFactId) mrr_identity.api.FactId.Insts.CoreCmpOrd
      mrr_identity.api.FactId.Insts.CoreCmpOrd entries id = .ok (reverseLookup entries id) := by
  change borrowedLookup core.borrow.Borrow.Blanket.borrow
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq id entries = _
  induction entries with
  | nil => rfl
  | cons pair rest induction =>
    cases pair with
    | mk key value =>
      simp only [borrowedLookup, core.borrow.Borrow.Blanket.borrow,
        native_fact_id_equality_exact, bind_ok, reverseLookup, induction]
      by_cases same : key = id <;> simp [same]

theorem native_impact_empty (traversal : NativeImpact) (empty : traversal.pending.val = []) :
    state.advance_impact traversal = .ok (false, traversal) := by
  simp only [state.advance_impact, pop_identity_empty traversal.pending empty, bind_ok]
  rfl

/-- One actual production pop and iterator fold, including absent adjacency.
No assumed adapter-step law appears; the loop result is derived above. -/
theorem native_impact_step_exact (traversal : NativeImpact) (id : NativeFactId)
    (tail : List NativeFactId) (pending : traversal.pending.val.reverse = id :: tail)
    (capacity : tail.length + ((reverseLookup traversal.reverse id).getD []).length <= Usize.max) :
    exists next, state.advance_impact traversal = .ok (true, next) /\
      next.reverse = traversal.reverse /\
      next.invalidated = (enqueueModel ((reverseLookup traversal.reverse id).getD [])
        traversal.invalidated tail.reverse).1 /\
      next.pending.val = (enqueueModel ((reverseLookup traversal.reverse id).getD [])
        traversal.invalidated tail.reverse).2 := by
  have original : traversal.pending.val = tail.reverse ++ [id] := by
    apply List.reverse_injective
    simpa using pending
  apply Exists.elim (pop_identity_lifo traversal.pending tail.reverse id original)
  intro remaining remainingSpec
  have contents : remaining.val = tail.reverse := remainingSpec.2
  cases lookup : reverseLookup traversal.reverse id with
  | none =>
    refine Exists.intro { traversal with pending := remaining } (And.intro ?_ ?_)
    next =>
      simp only [state.advance_impact, remainingSpec.1, bind_ok, uncurry,
        native_reverse_lookup_exact, lookup]
      try rfl
    next => exact And.intro rfl (And.intro rfl contents)
  | some neighbors =>
    have bound : remaining.val.length + neighbors.length <= Usize.max := by
      simpa only [lookup, Option.getD_some, contents, List.length_reverse] using capacity
    apply Exists.elim (native_impact_enqueue_exact neighbors traversal.invalidated remaining bound)
    intro selected selectedSpec
    apply Exists.elim selectedSpec
    intro queued queueSpec
    refine Exists.intro { traversal with invalidated := selected, pending := queued }
      (And.intro ?_ (And.intro rfl ?_))
    next =>
      simp only [state.advance_impact, remainingSpec.1, bind_ok, uncurry,
        native_reverse_lookup_exact, lookup,
        SharedABTreeSet.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter,
        queueSpec.1]
      try rfl
    next => simpa only [Option.getD_some, contents] using queueSpec.2

theorem enqueue_model_pending_members (neighbors scheduled pending : List NativeFactId)
    (id : NativeFactId) :
    id inList (enqueueModel neighbors scheduled pending).2 <->
      id inList pending \/ (id inList neighbors /\ Not (id inList scheduled)) := by
  induction neighbors generalizing scheduled pending with
  | nil => simp [enqueueModel]
  | cons head rest induction =>
    simp only [enqueueModel]
    by_cases known : head inList scheduled
    case pos =>
      simp only [known, if_true, induction, List.mem_cons]
      by_cases same : id = head
      case pos => simp [same, known]
      case neg => simp [same]
    case neg =>
      simp only [known, if_false, induction, List.mem_cons, List.mem_append]
      by_cases same : id = head
      case pos => simp [same, known]
      case neg => simp [same]

theorem native_nodup_subset_length (ids source : List NativeFactId)
    (unique : ids.Nodup) (subset : forall id, id inList ids -> id inList source) :
    ids.length <= source.length := by
  induction ids generalizing source with
  | nil => simp
  | cons head rest induction =>
    have distinct := List.nodup_cons.mp unique
    have found : head inList source := subset head (by simp)
    have smaller : forall id, id inList rest -> id inList source.erase head := by
      intro id member
      have different : Not (id = head) := by intro equal; subst id; exact distinct.1 member
      exact (List.mem_erase_of_ne different).mpr (subset id (by simp [member]))
    have bound := induction (source.erase head) distinct.2 smaller
    have removed := List.length_erase_of_mem found
    have positive : 0 < source.length := by
      cases source with
      | nil => simp at found
      | cons id remaining => simp
    simp only [List.length_cons]
    omega

abbrev NativeReverse := List (Prod NativeFactId (List NativeFactId))

def reverseNeighbors (entries : NativeReverse) (id : NativeFactId) : List NativeFactId :=
  (reverseLookup entries id).getD []

structure ReverseSource (source : List NativeFactId) (entries : NativeReverse)
    (deps : Nat -> List Nat) : Prop where
  neighborsUnique : forall id, (reverseNeighbors entries id).Nodup
  closed : forall parent, parent inList source -> forall id,
    id inList reverseNeighbors entries parent -> id inList source
  graph : forall id, deps (factModelId id) = (reverseNeighbors entries id).map factModelId
  capacity : source.length + source.length <= Usize.max

structure ReverseInvariant (source : List NativeFactId) (roots : List Nat)
    (entries : NativeReverse) (deps : Nat -> List Nat) (traversal : NativeImpact) : Prop where
  reverse : traversal.reverse = entries
  unique : traversal.invalidated.Nodup
  subset : forall id, id inList traversal.invalidated -> id inList source
  queued : forall id, id inList traversal.pending.val -> id inList traversal.invalidated
  slack : traversal.pending.val.length <= traversal.invalidated.length
  sound : forall id, id inList traversal.invalidated -> Required roots deps (factModelId id)
  roots : forall id, id inList roots -> id inList traversal.invalidated.map factModelId
  edges : forall parent, parent inList traversal.invalidated ->
    Not (parent inList traversal.pending.val) -> forall id,
    id inList reverseNeighbors entries parent -> id inList traversal.invalidated

def reverseRank (source : List NativeFactId) (traversal : NativeImpact) : Nat :=
  source.length - traversal.invalidated.length + traversal.pending.val.length

/-- The actual nonempty reverse step preserves the reachable frontier and
strictly decreases finite capacity. No assumed adapter law is supplied. -/
theorem native_impact_nonempty_preserves (source : List NativeFactId) (roots : List Nat)
    (entries : NativeReverse) (deps : Nat -> List Nat)
    (configuration : ReverseSource source entries deps) (traversal : NativeImpact)
    (valid : ReverseInvariant source roots entries deps traversal)
    (head : NativeFactId) (tail : List NativeFactId)
    (pending : traversal.pending.val.reverse = head :: tail) :
    exists next, state.advance_impact traversal = .ok (true, next) /\
      ReverseInvariant source roots entries deps next /\
      reverseRank source next < reverseRank source traversal := by
  let neighbors := reverseNeighbors entries head
  have headQueued : head inList traversal.pending.val :=
    List.mem_reverse.mp (by rw [pending]; exact List.mem_cons_self)
  have headSelected := valid.queued head headQueued
  have headSource := valid.subset head headSelected
  have oldBound := native_nodup_subset_length traversal.invalidated source valid.unique valid.subset
  have neighborBound := native_nodup_subset_length neighbors source
    (configuration.neighborsUnique head) (configuration.closed head headSource)
  have pendingLength : traversal.pending.val.length = tail.length + 1 := by
    simpa only [List.length_reverse, List.length_cons] using congrArg List.length pending
  have queueSlack := valid.slack
  have capacity : tail.length + ((reverseLookup traversal.reverse head).getD []).length <= Usize.max := by
    have maximum := configuration.capacity
    change neighbors.length <= source.length at neighborBound
    simpa only [valid.reverse, neighbors, reverseNeighbors] using (show tail.length + neighbors.length <= Usize.max by omega)
  apply Exists.elim (native_impact_step_exact traversal head tail pending capacity)
  intro next spec
  have selectedEq : next.invalidated = (enqueueModel neighbors traversal.invalidated tail.reverse).1 := by
    simpa only [valid.reverse, neighbors, reverseNeighbors] using spec.2.2.1
  have queueEq : next.pending.val = (enqueueModel neighbors traversal.invalidated tail.reverse).2 := by
    simpa only [valid.reverse, neighbors, reverseNeighbors] using spec.2.2.2
  have selectedMembers : forall id, id inList next.invalidated <->
      id inList traversal.invalidated \/ id inList neighbors := by
    intro id; rw [selectedEq]; exact enqueue_model_members _ _ _ id
  have queueMembers : forall id, id inList next.pending.val <->
      id inList tail \/ (id inList neighbors /\ Not (id inList traversal.invalidated)) := by
    intro id; rw [queueEq]; simpa only [List.mem_reverse] using enqueue_model_pending_members neighbors traversal.invalidated tail.reverse id
  have tailQueued : forall id, id inList tail -> id inList traversal.pending.val := by
    intro id member
    exact List.mem_reverse.mp (by rw [pending]; exact List.mem_cons_of_mem _ member)
  have balance : next.invalidated.length + tail.length =
      traversal.invalidated.length + next.pending.val.length := by
    simpa only [selectedEq, queueEq, List.length_reverse] using
      enqueue_model_length_balance neighbors traversal.invalidated tail.reverse
  have newUnique : next.invalidated.Nodup := by
    rw [selectedEq]; exact enqueue_model_selected_nodup _ _ _ valid.unique
  have newSubset : forall id, id inList next.invalidated -> id inList source := by
    intro id member
    cases (selectedMembers id).mp member with
    | inl old => exact valid.subset id old
    | inr added => exact configuration.closed head headSource id added
  have nextValid : ReverseInvariant source roots entries deps next :=
    { reverse := spec.2.1.trans valid.reverse
      unique := newUnique
      subset := newSubset
      queued := by
        intro id member
        apply (selectedMembers id).mpr
        cases (queueMembers id).mp member with
        | inl old => exact Or.inl (valid.queued id (tailQueued id old))
        | inr added => exact Or.inr added.1
      slack := by omega
      sound := by
        intro id member
        cases (selectedMembers id).mp member with
        | inl old => exact valid.sound id old
        | inr added =>
          apply Required.dependency (valid.sound head headSelected)
          rw [configuration.graph head]
          exact List.mem_map.mpr (Exists.intro id (And.intro added rfl))
      roots := by
        intro id member
        apply Exists.elim (List.mem_map.mp (valid.roots id member))
        intro actual actualSpec
        exact List.mem_map.mpr (Exists.intro actual
          (And.intro ((selectedMembers actual).mpr (Or.inl actualSpec.1)) actualSpec.2))
      edges := by
        intro parent selected finished id edge
        have oldSelected : parent inList traversal.invalidated := by
          by_cases old : parent inList traversal.invalidated
          case pos => exact old
          case neg =>
            have added := (selectedMembers parent).mp selected
            have neighbor : parent inList neighbors := added.resolve_left old
            exact False.elim (finished ((queueMembers parent).mpr (Or.inr (And.intro neighbor old))))
        apply (selectedMembers id).mpr
        by_cases same : parent = head
        case pos => exact Or.inr (by simpa only [same] using edge)
        case neg =>
          apply Or.inl (valid.edges parent oldSelected ?_ id edge)
          intro queued
          have queuedModel := List.mem_reverse.mpr queued
          rw [pending] at queuedModel
          cases List.mem_cons.mp queuedModel with
          | inl equal => exact same equal
          | inr member => exact finished ((queueMembers parent).mpr (Or.inl member)) }
  have newBound := native_nodup_subset_length next.invalidated source newUnique newSubset
  refine Exists.intro next (And.intro spec.1 (And.intro nextValid ?_))
  simp only [reverseRank]
  omega

def nativeImpactAdapter : worklist.Worklist NativeImpact :=
  state.ImpactTraversal.Insts.Mrr_agentic_ai_contextWorklistWorklist

theorem native_impact_advance_contract (source : List NativeFactId) (roots : List Nat)
    (entries : NativeReverse) (deps : Nat -> List Nat)
    (configuration : ReverseSource source entries deps) :
    AdvanceContract nativeImpactAdapter (ReverseInvariant source roots entries deps)
      (fun traversal => traversal.pending.val = []) (reverseRank source) := by
  intro traversal valid
  cases pending : traversal.pending.val.reverse with
  | nil =>
    have empty : traversal.pending.val = [] := by
      simpa only [List.reverse_reverse, List.reverse_nil] using congrArg List.reverse pending
    exact Exists.intro false (Exists.intro traversal (And.intro (native_impact_empty traversal empty)
      (And.intro valid (And.intro (by intro impossible; cases impossible) (fun _ => empty)))))
  | cons head tail =>
    apply Exists.elim (native_impact_nonempty_preserves source roots entries deps configuration traversal valid head tail pending)
    intro next spec
    exact Exists.intro true (Exists.intro next (And.intro spec.1
      (And.intro spec.2.1 (And.intro (fun _ => spec.2.2) (by intro impossible; cases impossible)))))

/-- Universal total correctness of the actual reverse loop under the named
library enumeration model and admitted reverse-index graph. -/
theorem native_impact_run_exact (source : List NativeFactId) (roots : List Nat)
    (entries : NativeReverse) (deps : Nat -> List Nat)
    (configuration : ReverseSource source entries deps) (initial : NativeImpact)
    (valid : ReverseInvariant source roots entries deps initial) :
    exists final, state.run_impact initial = .ok final /\ final.pending.val = [] /\
      forall id : NativeFactId, id inList final.invalidated <-> Required roots deps (factModelId id) := by
  apply Exists.elim (extracted_driver_preserves_invariant nativeImpactAdapter
    (ReverseInvariant source roots entries deps) (fun traversal => traversal.pending.val = [])
    (reverseRank source) (native_impact_advance_contract source roots entries deps configuration) initial valid)
  intro final spec
  refine Exists.intro final (And.intro spec.1 (And.intro spec.2.2 ?_))
  intro id
  constructor
  next => exact spec.2.1.sound id
  next =>
    intro required
    apply (selected_model_member final.invalidated id).mp
    apply closed_contains_required roots (final.invalidated.map factModelId) deps spec.2.1.roots ?_ _ required
    intro parent member child edge
    apply Exists.elim (List.mem_map.mp member)
    intro actual actualSpec
    rw [<- actualSpec.2, configuration.graph actual] at edge
    apply Exists.elim (List.mem_map.mp edge)
    intro neighbor neighborSpec
    exact List.mem_map.mpr (Exists.intro neighbor (And.intro
      (spec.2.1.edges actual actualSpec.1 (by simp only [spec.2.2, List.not_mem_nil, not_false_eq_true])
        neighbor neighborSpec.1) neighborSpec.2))

end MRR.ContextRustProofs
