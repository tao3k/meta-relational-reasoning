import Reverse

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

/-- Extensional update: preserve every other key and suppress repeated parents. -/
def putReverse : NativeReverse -> NativeFactId -> NativeFactId -> NativeReverse
  | [], dependency, parent => [(dependency, [parent])]
  | (key, parents) :: rest, dependency, parent =>
    if key = dependency then
      (key, if parent inList parents then parents else parent :: parents) :: rest
    else (key, parents) :: putReverse rest dependency parent

def addDeclared : List NativeFactId -> NativeReverse -> NativeFactId -> NativeReverse
  | [], entries, _ => entries
  | dependency :: rest, entries, parent =>
    addDeclared rest (putReverse entries dependency parent) parent

def indexDeclared : NativeElements -> NativeReverse -> NativeReverse
  | [], entries => entries
  | (parent, element) :: rest, entries =>
    indexDeclared rest (addDeclared element.dependencies.val entries parent)

abbrev NativeEntry := MRR.ContextRust.alloc.collections.btree.map.entry.Entry
  NativeFactId (List NativeFactId) Global

theorem native_entry_cursor_returns (entries : NativeReverse) (dependency : NativeFactId) :
    exists token : NativeEntry, exists restore : NativeEntry -> NativeReverse,
      entryCursor (A := Global) mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq
        dependency entries = .ok (token, restore) := by
  induction entries with
  | nil => exact Exists.intro _ (Exists.intro _ rfl)
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      by_cases same : key = dependency
      case pos =>
        refine Exists.intro (.Occupied (key, parents)) (Exists.intro
          (fun updated => match updated with
            | .Vacant _ => (key, parents) :: rest
            | .Occupied (_, values) => (key, values) :: rest) ?_)
        simp [entryCursor, native_fact_id_equality_exact, same]
        funext token
        cases token with
        | Vacant keyValue => rfl
        | Occupied pairValue => cases pairValue; rfl
      case neg =>
        apply Exists.elim induction
        intro token tokenSpec
        apply Exists.elim tokenSpec
        intro restore cursor
        refine Exists.intro token (Exists.intro (fun updated => (key, parents) :: restore updated) ?_)
        simp [entryCursor, native_fact_id_equality_exact, same, cursor]

/-- A single actual borrowed entry/default/insert transaction restores the map
frame and has exactly the declared extensional update. -/
theorem native_reverse_edge_exact (entries : NativeReverse)
    (dependency parent : NativeFactId) :
    state.insert_reverse_edge entries dependency parent =
      .ok (putReverse entries dependency parent) := by
  induction entries with
  | nil =>
    simp only [state.insert_reverse_edge, alloc.collections.btree.map.BTreeMap.entry,
      entryCursor, bind_ok, uncurry, alloc.collections.btree.map.entry.Entry.or_default,
      alloc.collections.btree.set.BTreeSetTGlobal.Insts.CoreDefaultDefault.default,
      native_set_insert_exact, List.not_mem_nil, if_false,
      putReverse]
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      simp only [state.insert_reverse_edge, alloc.collections.btree.map.BTreeMap.entry,
        entryCursor, native_fact_id_equality_exact, bind_ok]
      by_cases same : key = dependency
      case pos =>
        simp only [same, if_true, bind_ok, uncurry,
          alloc.collections.btree.map.entry.Entry.or_default, native_set_insert_exact,
          putReverse, if_true]
        by_cases known : parent inList parents <;> simp [known]
      case neg =>
        simp only [same, decide_false, Bool.false_eq_true, if_false, putReverse]
        apply Exists.elim (native_entry_cursor_returns rest dependency)
        intro token tokenSpec
        apply Exists.elim tokenSpec
        intro restore cursor
        simp only [state.insert_reverse_edge, alloc.collections.btree.map.BTreeMap.entry,
          cursor, bind_ok, uncurry] at induction
        simp only [cursor, bind_ok, uncurry]
        cases token with
        | Vacant keyValue =>
          simp only [alloc.collections.btree.map.entry.Entry.or_default,
            alloc.collections.btree.set.BTreeSetTGlobal.Insts.CoreDefaultDefault.default,
            bind_ok, uncurry, native_set_insert_exact, List.not_mem_nil, if_false] at induction
          simp only [alloc.collections.btree.map.entry.Entry.or_default,
            alloc.collections.btree.set.BTreeSetTGlobal.Insts.CoreDefaultDefault.default,
            bind_ok, uncurry, native_set_insert_exact, List.not_mem_nil, if_false]
          exact congrArg (fun reverse => Result.ok ((key, parents) :: reverse)) (Result.ok_injective induction)
        | Occupied pairValue =>
          cases pairValue with
          | mk keyValue values =>
            simp only [alloc.collections.btree.map.entry.Entry.or_default,
              bind_ok, uncurry, native_set_insert_exact] at induction
            simp only [alloc.collections.btree.map.entry.Entry.or_default,
              bind_ok, uncurry, native_set_insert_exact]
            by_cases known : parent inList values <;>
              simp only [known, if_true, if_false] at * <;>
              exact congrArg (fun reverse => Result.ok ((key, parents) :: reverse)) (Result.ok_injective induction)

theorem put_reverse_members (entries : NativeReverse)
    (dependency parent query observer : NativeFactId) :
    observer inList reverseNeighbors (putReverse entries dependency parent) query <->
      observer inList reverseNeighbors entries query \/
        (query = dependency /\ observer = parent) := by
  induction entries with
  | nil =>
    by_cases same : dependency = query
    case pos => simp [putReverse, reverseNeighbors, reverseLookup, same]
    case neg => simp [putReverse, reverseNeighbors, reverseLookup, same, Ne.symm same]
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      by_cases updated : key = dependency
      case pos =>
        subst key
        by_cases queried : dependency = query
        case pos =>
          subst query
          by_cases known : parent inList parents
          case pos =>
            simp only [putReverse, if_true, known, reverseNeighbors, reverseLookup,
              Option.getD_some]
            constructor
            next => intro member; exact Or.inl member
            next => intro member; cases member with
                    | inl old => exact old
                    | inr equal => simpa only [equal] using known
          case neg => simp [putReverse, reverseNeighbors, reverseLookup, known, or_comm]
        case neg => simp [putReverse, reverseNeighbors, reverseLookup, queried, Ne.symm queried]
      case neg =>
        by_cases queried : key = query
        case pos =>
          subst query
          simp [putReverse, reverseNeighbors, reverseLookup, updated]
        case neg =>
          simpa only [putReverse, updated, if_false, reverseNeighbors, reverseLookup,
            queried, Option.getD] using induction

theorem add_declared_members (dependencies : List NativeFactId) (entries : NativeReverse)
    (parent query observer : NativeFactId) :
    observer inList reverseNeighbors (addDeclared dependencies entries parent) query <->
      observer inList reverseNeighbors entries query \/
        (observer = parent /\ query inList dependencies) := by
  induction dependencies generalizing entries with
  | nil => simp [addDeclared]
  | cons dependency rest induction =>
    simp only [addDeclared, induction, put_reverse_members, List.mem_cons]
    tauto

theorem index_declared_members (elements : NativeElements) (entries : NativeReverse)
    (query observer : NativeFactId) :
    observer inList reverseNeighbors (indexDeclared elements entries) query <->
      observer inList reverseNeighbors entries query \/
        exists element, (observer, element) inList elements /\ query inList element.dependencies.val := by
  induction elements generalizing entries with
  | nil => simp [indexDeclared]
  | cons pair rest induction =>
    cases pair with
    | mk parent element =>
      simp only [indexDeclared, induction, add_declared_members, List.mem_cons]
      constructor
      next =>
        intro member
        cases member with
        | inl previous =>
          cases previous with
          | inl old => exact Or.inl old
          | inr added =>
            exact Or.inr (Exists.intro element (And.intro
              (Or.inl (by simp only [added.1])) added.2))
        | inr added =>
          apply Or.inr
          apply Exists.elim added
          intro value valueSpec
          exact Exists.intro value (And.intro (Or.inr valueSpec.1) valueSpec.2)
      next =>
        intro member
        cases member with
        | inl old => exact Or.inl (Or.inl old)
        | inr added =>
          apply Exists.elim added
          intro value valueSpec
          cases valueSpec.1 with
          | inl equal =>
            have parentEq : observer = parent := congrArg Prod.fst equal
            have elementEq : value = element := congrArg Prod.snd equal
            exact Or.inl (Or.inr (And.intro parentEq (by simpa only [elementEq] using valueSpec.2)))
          | inr old => exact Or.inr (Exists.intro value (And.intro old valueSpec.2))

/-- Total correctness of the actual inner dependency iterator, from any suffix. -/
theorem native_reverse_dependency_loop_exact (iter : core.slice.iter.Iter NativeFactId)
    (entries : NativeReverse) (parent : NativeFactId) :
    state.add_reverse_dependencies_loop0_loop0 iter entries parent =
      .ok (addDeclared (iter.slice.val.drop iter.i) entries parent) := by
  let expected := addDeclared (iter.slice.val.drop iter.i) entries parent
  let invariant := fun input : Prod (core.slice.iter.Iter NativeFactId) NativeReverse =>
    addDeclared (input.1.slice.val.drop input.1.i) input.2 parent = expected
  have total : WP.spec (loop
      (fun (current, reverse) => state.add_reverse_dependencies_loop0_loop0.body parent current reverse)
      (iter, entries)) (fun final => final = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.slice.val.length - input.1.i)
      invariant (fun final => final = expected)
    next =>
      intro input valid
      cases input
      rename_i current reverse
      have exactModel := valid
      simp only [state.add_reverse_dependencies_loop0_loop0.body,
        core.slice.iter.IteratorSliceIter.next]
      by_cases active : current.i < current.slice.len
      case pos =>
        simp only [dif_pos active, bind_ok, uncurry,
          native_reverse_edge_exact]
        apply (WP.spec_ok _).mpr
        refine And.intro ?_ ?_
        next =>
          have indexBound : current.i < current.slice.val.length := by
            simpa only [Slice.len_val] using active
          change addDeclared (current.slice.val.drop (current.i + 1))
            (putReverse reverse current.slice.val[current.i] parent) parent = expected
          change addDeclared (current.slice.val.drop current.i) reverse parent = expected at exactModel
          rw [List.drop_eq_getElem_cons indexBound] at exactModel
          exact exactModel
        next =>
          have indexBound : current.i < current.slice.val.length := by
            simpa only [Slice.len_val] using active
          change current.slice.val.length - (current.i + 1) < current.slice.val.length - current.i
          omega
      case neg =>
        simp only [dif_neg active, bind_ok, uncurry]
        apply (WP.spec_ok _).mpr
        have beyond : current.slice.val.length <= current.i := by
          have inactive : Not (current.i < current.slice.val.length) := by
            simpa only [Slice.len_val] using active
          omega
        change addDeclared (current.slice.val.drop current.i) reverse parent = expected at exactModel
        simpa only [List.drop_eq_nil_iff.mpr beyond, addDeclared] using exactModel
    next => rfl
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro final spec
  exact spec.1.trans (congrArg Result.ok spec.2)

/-- Total correctness of the actual outer map enumeration. -/
theorem native_reverse_elements_loop_exact (elements : NativeElements) (entries : NativeReverse) :
    state.add_reverse_dependencies_loop0 elements entries = .ok (indexDeclared elements entries) := by
  let expected := indexDeclared elements entries
  let invariant := fun input : Prod NativeElements NativeReverse => indexDeclared input.1 input.2 = expected
  have total : WP.spec (loop
      (fun (iter, reverse) => state.add_reverse_dependencies_loop0.body iter reverse)
      (elements, entries)) (fun final => final = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.length) invariant (fun final => final = expected)
    next =>
      intro input valid
      cases input
      rename_i current reverse
      cases current with
      | nil =>
        simp only [state.add_reverse_dependencies_loop0.body,
          alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next,
          bind_ok, uncurry]
        exact (WP.spec_ok _).mpr valid
      | cons pair rest =>
        cases pair with
        | mk parent element =>
          simp only [state.add_reverse_dependencies_loop0.body,
            alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next,
            bind_ok, uncurry, core.slice.Slice.iter,
            native_reverse_dependency_loop_exact, alloc.vec.Vec.deref, Slice.from_val,
            List.drop_zero]
          apply (WP.spec_ok _).mpr
          exact And.intro valid (by simp only [List.length_cons]; omega)
    next => rfl
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro final spec
  exact spec.1.trans (congrArg Result.ok spec.2)

theorem native_reverse_add_exact (elements : NativeElements) (entries : NativeReverse) :
    state.add_reverse_dependencies elements entries = .ok (indexDeclared elements entries) := by
  simp only [state.add_reverse_dependencies,
    SharedABTreeMap.Insts.CoreIterTraitsCollectIntoIteratorPairSharedAKSharedAVIter.into_iter,
    bind_ok, native_reverse_elements_loop_exact]

theorem native_reverse_index_exact (old new : NativeElements) :
    state.build_reverse_index old new = .ok (indexDeclared new (indexDeclared old [])) := by
  simp only [state.build_reverse_index, alloc.collections.btree.map.BTreeMapKVGlobal.new,
    bind_ok, native_reverse_add_exact]

def ReverseValuesUnique (entries : NativeReverse) : Prop :=
  forall key parents, (key, parents) inList entries -> parents.Nodup

theorem put_reverse_values_unique (entries : NativeReverse) (dependency parent : NativeFactId)
    (unique : ReverseValuesUnique entries) : ReverseValuesUnique (putReverse entries dependency parent) := by
  induction entries with
  | nil => simp [ReverseValuesUnique, putReverse]
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      have currentUnique : parents.Nodup := unique key parents (by simp)
      have restUnique : ReverseValuesUnique rest := by
        intro k values member; exact unique k values (List.mem_cons_of_mem _ member)
      have tailUnique := induction restUnique
      by_cases updated : key = dependency
      case pos =>
        simp only [putReverse, updated, if_true]
        by_cases known : parent inList parents
        case pos =>
          simp only [known, if_true]
          simpa only [updated] using unique
        case neg =>
          simp only [known, if_false]
          intro k values member
          cases List.mem_cons.mp member with
          | inl equal =>
            have same : values = parent :: parents := congrArg Prod.snd equal
            rw [same]; exact List.nodup_cons.mpr (And.intro known currentUnique)
          | inr old => exact restUnique k values old
      case neg =>
        simp only [putReverse, updated, if_false]
        intro k values member
        cases List.mem_cons.mp member with
        | inl equal =>
          have sameValues : values = parents := congrArg Prod.snd equal
          rw [sameValues]; exact currentUnique
        | inr old => exact tailUnique k values old

theorem add_declared_values_unique (dependencies : List NativeFactId) (entries : NativeReverse)
    (parent : NativeFactId) (unique : ReverseValuesUnique entries) :
    ReverseValuesUnique (addDeclared dependencies entries parent) := by
  induction dependencies generalizing entries with
  | nil => exact unique
  | cons dependency rest induction =>
    exact induction _ (put_reverse_values_unique entries dependency parent unique)

theorem index_declared_values_unique (elements : NativeElements) (entries : NativeReverse)
    (unique : ReverseValuesUnique entries) : ReverseValuesUnique (indexDeclared elements entries) := by
  induction elements generalizing entries with
  | nil => exact unique
  | cons pair rest induction =>
    cases pair with
    | mk parent element =>
      exact induction _ (add_declared_values_unique element.dependencies.val entries parent unique)

theorem reverse_values_unique_neighbors (entries : NativeReverse)
    (unique : ReverseValuesUnique entries) (id : NativeFactId) : (reverseNeighbors entries id).Nodup := by
  induction entries with
  | nil => simp [reverseNeighbors, reverseLookup]
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      by_cases same : key = id
      case pos =>
        simpa only [reverseNeighbors, reverseLookup, same, if_true, Option.getD_some] using
          unique key parents (by simp)
      case neg =>
        simp only [reverseNeighbors, reverseLookup, same, if_false]
        exact induction (by intro k values member; exact unique k values (List.mem_cons_of_mem _ member))

/-- The actual old/new builder creates exactly the union of declared reverse
edges, with duplicate-free neighbor values. No graph correspondence premise is
needed for this construction theorem. -/
theorem native_reverse_index_declared_union (old new : NativeElements) :
    exists entries, state.build_reverse_index old new = .ok entries /\
      (forall query observer, observer inList reverseNeighbors entries query <->
        (exists element, (observer, element) inList old /\ query inList element.dependencies.val) \/
        (exists element, (observer, element) inList new /\ query inList element.dependencies.val)) /\
      (forall id, (reverseNeighbors entries id).Nodup) := by
  refine Exists.intro (indexDeclared new (indexDeclared old []))
    (And.intro (native_reverse_index_exact old new) (And.intro ?_ ?_))
  next =>
    intro query observer
    rw [index_declared_members, index_declared_members]
    simp only [reverseNeighbors, reverseLookup, Option.getD_none, List.not_mem_nil, false_or]
  next =>
    intro id
    exact reverse_values_unique_neighbors _
      (index_declared_values_unique new _ (index_declared_values_unique old []
        (by intro key values member; cases member))) id

def modelReverseGraph : NativeReverse -> Nat -> List Nat
  | [], _ => []
  | (key, parents) :: rest, id =>
    if factModelId key = id then parents.map factModelId else modelReverseGraph rest id

theorem native_reverse_graph_projection (entries : NativeReverse) (id : NativeFactId) :
    modelReverseGraph entries (factModelId id) = (reverseNeighbors entries id).map factModelId := by
  induction entries with
  | nil => rfl
  | cons pair rest induction =>
    cases pair with
    | mk key parents =>
      by_cases same : key = id
      case pos => simp [modelReverseGraph, reverseNeighbors, reverseLookup, same]
      case neg =>
        have different : Not (factModelId key = factModelId id) := fun equal => same (fact_model_id_injective equal)
        simp [modelReverseGraph, reverseNeighbors, reverseLookup, same, different, induction]

theorem native_impact_seed_invariant (source changed : List NativeFactId)
    (entries : NativeReverse) (deps : Nat -> List Nat) (pending : alloc.vec.Vec NativeFactId)
    (contents : pending.val = changed) (unique : changed.Nodup)
    (subset : forall id, id inList changed -> id inList source) :
    ReverseInvariant source (changed.map factModelId) entries deps
      { reverse := entries, invalidated := changed, pending := pending } := by
  refine ReverseInvariant.mk rfl unique subset ?_ ?_ ?_ ?_ ?_
  next => intro id member; simpa only [contents] using member
  next => simp only [contents, Nat.le_refl]
  next => intro id member; exact Required.root (List.mem_map.mpr (Exists.intro id (And.intro member rfl)))
  next => intro id member; exact member
  next => intro parent member finished; exact False.elim (finished (by simpa only [contents] using member))

/-- Connect the actual old/new index builder to the actual reverse driver.
The source graph and initial invariant are derived here. Only the seed Vec
contents, changed-set uniqueness and finite machine budget remain premises. -/
theorem native_declared_impact_run_exact (old new : NativeElements) (changed : List NativeFactId)
    (pending : alloc.vec.Vec NativeFactId) (contents : pending.val = changed)
    (unique : changed.Nodup)
    (capacity : (changed ++ (old ++ new).map Prod.fst).length +
      (changed ++ (old ++ new).map Prod.fst).length <= Usize.max) :
    exists entries final, state.build_reverse_index old new = .ok entries /\
      state.run_impact { reverse := entries, invalidated := changed, pending := pending } = .ok final /\
      final.pending.val = [] /\ forall id : NativeFactId,
        id inList final.invalidated <-> Required (changed.map factModelId) (modelReverseGraph entries) (factModelId id) := by
  apply Exists.elim (native_reverse_index_declared_union old new)
  intro entries indexSpec
  let source := changed ++ (old ++ new).map Prod.fst
  have declaredSource : forall parent query, parent inList reverseNeighbors entries query -> parent inList source := by
    intro parent query member
    have declaration := (indexSpec.2.1 query parent).mp member
    apply List.mem_append.mpr
    apply Or.inr
    cases declaration with
    | inl oldMember =>
      apply Exists.elim oldMember
      intro element elementSpec
      exact List.mem_map.mpr (Exists.intro (parent, element)
        (And.intro (List.mem_append.mpr (Or.inl elementSpec.1)) rfl))
    | inr newMember =>
      apply Exists.elim newMember
      intro element elementSpec
      exact List.mem_map.mpr (Exists.intro (parent, element)
        (And.intro (List.mem_append.mpr (Or.inr elementSpec.1)) rfl))
  have configuration : ReverseSource source entries (modelReverseGraph entries) :=
    { neighborsUnique := indexSpec.2.2
      closed := fun parent _ id member => declaredSource id parent member
      graph := native_reverse_graph_projection entries
      capacity := capacity }
  have seeded := native_impact_seed_invariant source changed entries (modelReverseGraph entries)
    pending contents unique (fun id member => List.mem_append.mpr (Or.inl member))
  apply Exists.elim (native_impact_run_exact source (changed.map factModelId) entries (modelReverseGraph entries)
    configuration { reverse := entries, invalidated := changed, pending := pending } seeded)
  intro final finalSpec
  exact Exists.intro entries (Exists.intro final (And.intro indexSpec.1 finalSpec))

end MRR.ContextRustProofs
