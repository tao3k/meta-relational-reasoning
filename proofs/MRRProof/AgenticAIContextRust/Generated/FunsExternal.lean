-- Trusted standard-library value models, not Rust unsafe implementation proofs.
-- Successful allocation and lawful identity equality remain library assumptions.
-- get/insert model extensional membership only; no iteration/sort law is claimed.
module
public import Aeneas
public import Generated.Types
@[expose] public section
open Aeneas.Std
open MRR.ContextRust
local infixr:35 " ** " => Prod

namespace Aeneas.Std

def compareValues {T : Type} (cmp : T -> T -> Result Ordering) :
    List T -> List T -> Result Ordering
  | [], [] => .ok .eq
  | [], _ :: _ => .ok .lt
  | _ :: _, [] => .ok .gt
  | left :: ls, right :: rs => do
    let order <- cmp left right
    match order with
    | .eq => compareValues cmp ls rs
    | other => .ok other

def Array.Insts.CoreCmpOrd.cmp {T : Type} {N : Usize}
    (order : core.cmp.Ord T) (left right : Array T N) : Result Ordering :=
  compareValues order.cmp left.val right.val

def borrowedLookup {K V Q : Type} (borrow : K -> Result Q)
    (equal : Q -> Q -> Result Bool) (query : Q) : List (K ** V) -> Result (Option V)
  | [] => .ok none
  | (key, value) :: rest => do
    let borrowed <- borrow key
    let matched <- equal borrowed query
    if matched then .ok (some value)
    else borrowedLookup borrow equal query rest

def containsValue {T : Type} (equal : T -> T -> Result Bool)
    (query : T) : List T -> Result Bool
  | [] => .ok false
  | value :: rest => do
    let matched <- equal value query
    if matched then .ok true else containsValue equal query rest

def alloc.collections.btree.map.BTreeMap.get
    {K V A Q : Type} (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A)
    (borrow : MRR.ContextRust.core.borrow.Borrow K Q)
    (_keyOrder : core.cmp.Ord K) (queryOrder : core.cmp.Ord Q)
    (map : alloc.collections.btree.map.BTreeMap K V A) (query : Q) : Result (Option V) :=
  borrowedLookup borrow.borrow queryOrder.eqInst.partialEqInst.eq query map

def alloc.collections.btree.set.BTreeSet.insert {T A : Type}
    (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A) (order : core.cmp.Ord T)
    (set : alloc.collections.btree.set.BTreeSet T A) (value : T) :
    Result (Bool ** alloc.collections.btree.set.BTreeSet T A) := do
  let known <- containsValue order.eqInst.partialEqInst.eq value set
  if known then .ok (false, set) else .ok (true, value :: set)

-- Trusted enumeration without duplicates for a well-formed extensional set.
-- No correspondence with unsafe iterator implementation or sorted order is proved.
def SharedABTreeSet.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter
    {T A : Type} (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A)
    (set : alloc.collections.btree.set.BTreeSet T A) :
    Result (alloc.collections.btree.set.Iter T) := .ok set

def alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next
    {T : Type} : alloc.collections.btree.set.Iter T ->
    Result (Option T ** alloc.collections.btree.set.Iter T)
  | [] => .ok (none, [])
  | id :: rest => .ok (some id, rest)

def alloc.collections.btree.set.BTreeSetTGlobal.new (T : Type) :
    Result (alloc.collections.btree.set.BTreeSet T Global) := .ok []

def alloc.collections.btree.set.BTreeSetTGlobal.Insts.CoreDefaultDefault.default
    (T : Type) : Result (alloc.collections.btree.set.BTreeSet T Global) := .ok []

def alloc.collections.btree.map.BTreeMapKVGlobal.new (K V : Type) :
    Result (alloc.collections.btree.map.BTreeMap K V Global) := .ok []

def SharedABTreeMap.Insts.CoreIterTraitsCollectIntoIteratorPairSharedAKSharedAVIter.into_iter
    {K V A : Type} (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A)
    (map : alloc.collections.btree.map.BTreeMap K V A) :
    Result (alloc.collections.btree.map.Iter K V) := .ok map

def alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next
    {K V : Type} : alloc.collections.btree.map.Iter K V ->
    Result (Option (K ** V) ** alloc.collections.btree.map.Iter K V)
  | [] => .ok (none, [])
  | pair :: rest => .ok (some pair, rest)

-- An entry carries a borrowed value; its return closure restores the same key
-- and preserves every other binding. These are trusted value/borrow models.
def entryCursor {K V A : Type} (equal : K -> K -> Result Bool) (query : K) :
    List (K ** V) -> Result (MRR.ContextRust.alloc.collections.btree.map.entry.Entry K V A **
      (MRR.ContextRust.alloc.collections.btree.map.entry.Entry K V A -> List (K ** V)))
  | [] => .ok (.Vacant query, fun entry => match entry with
      | .Vacant _ => []
      | .Occupied (_, value) => [(query, value)])
  | (key, value) :: rest => do
    let found <- equal key query
    if found then .ok (.Occupied (key, value), fun entry => match entry with
      | .Vacant _ => (key, value) :: rest
      | .Occupied (_, updated) => (key, updated) :: rest)
    else do
      let (entry, restore) <- entryCursor equal query rest
      .ok (entry, fun updated => (key, value) :: restore updated)

def alloc.collections.btree.map.BTreeMap.entry {K V A : Type}
    (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A) (order : core.cmp.Ord K)
    (map : alloc.collections.btree.map.BTreeMap K V A) (query : K) :=
  entryCursor (A := A) order.eqInst.partialEqInst.eq query map

def alloc.collections.btree.map.entry.Entry.or_default {K V A : Type}
    (_order : core.cmp.Ord K) (default : core.default.Default V)
    (_allocator : MRR.ContextRust.core.alloc.AllocatorClone A)
    (entry : MRR.ContextRust.alloc.collections.btree.map.entry.Entry K V A) :
    Result (V ** (V -> MRR.ContextRust.alloc.collections.btree.map.entry.Entry K V A)) :=
  match entry with
  | .Occupied (key, value) => .ok (value, fun updated => .Occupied (key, updated))
  | .Vacant key => do
      let value <- default.default
      .ok (value, fun updated => .Occupied (key, updated))

end Aeneas.Std
