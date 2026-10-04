-------------------------- MODULE RevisionLifecycle --------------------------
EXTENDS Naturals, FiniteSets
CONSTANTS Ids, OldEdges, NewEdges, GlobalChanged, Unequal, OldSelected, NewSelected,
          Bug
ASSUME /\ IsFiniteSet(Ids)
       /\ OldEdges \subseteq Ids \X Ids
       /\ NewEdges \subseteq Ids \X Ids
       /\ Unequal \subseteq Ids
       /\ OldSelected \subseteq Ids
       /\ NewSelected \subseteq Ids
       /\ GlobalChanged \in BOOLEAN
       /\ Bug \in {"none", "oldEdgesOnly", "publishEarly"}
VARIABLES phase, todo, changed, invalidated, reusable,
          source, oldEdges, newEdges, global, unequal, oldSelected, newSelected
inputs == <<source, oldEdges, newEdges, global, unequal, oldSelected, newSelected>>
vars == <<phase, todo, changed, invalidated, reusable, inputs>>
Edges == OldEdges \cup NewEdges
UsedEdges == IF Bug = "oldEdgesOnly" THEN OldEdges ELSE Edges
Changed == IF GlobalChanged THEN Ids ELSE Unequal
Users(edges, set) == {id \in Ids : \E dep \in set : <<id, dep>> \in edges}
RECURSIVE Reach(_, _)
Reach(set, n) == IF n = 0 THEN set ELSE
                  LET previous == Reach(set, n - 1)
                  IN previous \cup Users(Edges, previous)
Impact == Reach(Changed, Cardinality(Ids))
Init == /\ phase = "classify" /\ todo = Ids /\ changed = {}
        /\ invalidated = {} /\ reusable = {}
        /\ source = Ids /\ oldEdges = OldEdges /\ newEdges = NewEdges
        /\ global = GlobalChanged /\ unequal = Unequal
        /\ oldSelected = OldSelected /\ newSelected = NewSelected
Classify(id) == /\ phase = "classify" /\ id \in todo
               /\ todo' = todo \ {id}
               /\ changed' = (IF id \in Changed THEN changed \cup {id} ELSE changed)
               /\ UNCHANGED <<phase, invalidated, reusable>>
Seed == /\ phase = "classify" /\ todo = {}
        /\ phase' = "invalidate" /\ invalidated' = changed
        /\ UNCHANGED <<todo, changed, reusable>>
Grow(id) == /\ phase = "invalidate"
            /\ id \in Users(UsedEdges, invalidated) \ invalidated
            /\ invalidated' = invalidated \cup {id}
            /\ UNCHANGED <<phase, todo, changed, reusable>>
Seal == /\ phase = "invalidate"
        /\ (Users(UsedEdges, invalidated) \subseteq invalidated \/ Bug = "publishEarly")
        /\ phase' = "ready" /\ UNCHANGED <<todo, changed, invalidated, reusable>>
Publish == /\ phase = "ready" /\ phase' = "published"
           /\ reusable' = (OldSelected \cap NewSelected) \ invalidated
           /\ UNCHANGED <<todo, changed, invalidated>>
Next == ((\E id \in Ids : Classify(id) \/ Grow(id)) \/ Seed \/ Seal \/ Publish) /\ UNCHANGED inputs
Spec == Init /\ [][Next]_vars /\ WF_vars(Next)
TypeOK == /\ phase \in {"classify", "invalidate", "ready", "published"}
          /\ todo \subseteq Ids /\ changed \subseteq Ids
          /\ invalidated \subseteq Ids /\ reusable \subseteq Ids
Classification == changed = Changed \ (IF phase = "classify" THEN todo ELSE {})
ImpactSound == invalidated \subseteq Impact
CompleteBeforePublish == phase \in {"ready", "published"} => invalidated = Impact
ReuseSafe == phase = "published" =>
               /\ reusable = (OldSelected \cap NewSelected) \ Impact
               /\ (GlobalChanged => reusable = {})
EventuallyPublished == <> (phase = "published")
PublishedStable == [][phase = "published" => UNCHANGED <<invalidated, reusable>>]_vars
=============================================================================
