------------------------------ MODULE MetaImpact ------------------------------
EXTENDS Naturals, FiniteSets
CONSTANTS Inputs, Branches, OldSupports, NewSupports, OldPresent, NewPresent,
          Admitted, SupportComplete, CoverageComplete, Observed, NegativeTarget,
          OwnerClaim, Bug
ASSUME /\ IsFiniteSet(Inputs) /\ IsFiniteSet(Branches)
       /\ OldSupports \subseteq Branches \X Inputs
       /\ NewSupports \subseteq Branches \X Inputs
       /\ OldPresent \subseteq Inputs /\ NewPresent \subseteq Inputs
       /\ Admitted \subseteq Branches /\ Observed \subseteq Inputs
       /\ NegativeTarget \in Inputs
       /\ SupportComplete \in BOOLEAN /\ CoverageComplete \in BOOLEAN
       /\ OwnerClaim \in {"structural", "temporal", "contrast", "effect"}
       /\ Bug \in {"none", "dropAlternative", "dropNewEdges",
                    "uncertifiedAbsence", "promoteCausal"}
VARIABLES phase, todo, live, frontier, verdict, absence, claim
vars == <<phase, todo, live, frontier, verdict, absence, claim>>
Changed == (OldPresent \ NewPresent) \cup (NewPresent \ OldPresent)
Touched(edges) == \E branch \in Branches : \E input \in Changed :
                    <<branch, input>> \in edges
Frontier == Touched(OldSupports \cup NewSupports)
UsedFrontier == Touched(IF Bug = "dropNewEdges" THEN OldSupports
                        ELSE OldSupports \cup NewSupports)
Ready(branch, edges, present) ==
  /\ branch \in Admitted
  /\ \E input \in Inputs : <<branch, input>> \in edges
  /\ \A input \in Inputs : <<branch, input>> \in edges => input \in present
OldAlive == \E branch \in Branches : Ready(branch, OldSupports, OldPresent)
NewAlive == \E branch \in Branches : Ready(branch, NewSupports, NewPresent)
Init == /\ phase = "scan" /\ todo = Branches /\ live = {}
        /\ frontier = FALSE /\ verdict = "pending"
        /\ absence = FALSE /\ claim = "pending"
Scan == /\ phase = "scan" /\ phase' = "evaluate"
        /\ frontier' = UsedFrontier
        /\ UNCHANGED <<todo, live, verdict, absence, claim>>
Evaluate(branch) == /\ phase = "evaluate" /\ branch \in todo
                    /\ todo' = todo \ {branch}
                    /\ live' = IF Ready(branch, NewSupports, NewPresent)
                        /\ ~(Bug = "dropAlternative" /\ branch = 1)
                       THEN live \cup {branch} ELSE live
                    /\ UNCHANGED <<phase, frontier, verdict, absence, claim>>
Seal == /\ phase = "evaluate" /\ todo = {}
        /\ phase' = "ready"
        /\ verdict' = IF live # {} THEN
                         (IF OldAlive THEN "revalidated" ELSE "added")
                       ELSE IF OldAlive /\ SupportComplete THEN "retracted"
                       ELSE "unknown"
        /\ absence' = IF Bug = "uncertifiedAbsence" THEN TRUE
                        ELSE CoverageComplete /\ NegativeTarget \notin Observed
        /\ claim' = IF Bug = "promoteCausal" THEN "effect" ELSE OwnerClaim
        /\ UNCHANGED <<todo, live, frontier>>
Publish == /\ phase = "ready" /\ phase' = "published"
           /\ UNCHANGED <<todo, live, frontier, verdict, absence, claim>>
Next == Scan \/ (\E branch \in Branches : Evaluate(branch)) \/ Seal \/ Publish
Spec == Init /\ [][Next]_vars /\ WF_vars(Next)
TypeOK == /\ phase \in {"scan", "evaluate", "ready", "published"}
          /\ todo \subseteq Branches /\ live \subseteq Branches
          /\ frontier \in BOOLEAN /\ absence \in BOOLEAN
          /\ verdict \in {"pending", "revalidated", "added", "retracted", "unknown"}
          /\ claim \in {"pending", "structural", "temporal", "contrast", "effect"}
FrontierSound == phase \in {"ready", "published"} => (Frontier => frontier)
NoFalseRetraction == phase = "published" => (verdict = "retracted" => ~NewAlive)
NoUncertifiedAbsence == phase = "published" =>
  (absence => CoverageComplete /\ NegativeTarget \notin Observed)
NoTemporalPromotion == phase = "published" => claim = OwnerClaim
SupportCompleteBeforePublish == phase = "published" => todo = {}
EventuallyPublished == <> (phase = "published")
=============================================================================
