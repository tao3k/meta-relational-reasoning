"""Kernel rejection fixtures against the actual indexed transformation contract."""
import pytest
from mrr_proof_validation import cli


def test_transformation_axiom_policy_rejects_missing_and_custom_axioms():
    source = (cli.ROOT / "proofs/MRRProof/Transformation.lean").read_text()
    receipt = cli.local_lean_check(source)
    assert receipt["transformationAxioms"]["MRRTransformation.compose_sound"] == []
    with pytest.raises(AssertionError, match="missing transformation axiom report"):
        cli.transformation_axiom_report("")
    with pytest.raises(AssertionError, match="unapproved transformation axioms"):
        cli.transformation_axiom_report("'MRRTransformation.compose_sound' depends on axioms: [forged]")


@pytest.mark.parametrize("invalid", [
    """
def wrongInstance (input : Nat) (answer : Fin (input + 2)) : Fin (input + 1) := answer
""",
    """
def missingPremise {A B : MRRTransformation.Problem}
    (f : MRRTransformation.CertifiedTransformation A B) (input : A.Input)
    (answer : B.Result (f.forward input))
    (correct : B.Correct (f.forward input) answer) :
    A.Correct input (f.extract input answer) := f.sound input answer correct
""",
    """
def incompatibleEndpoints {A B C D : MRRTransformation.Problem}
    (f : MRRTransformation.CertifiedTransformation A B)
    (g : MRRTransformation.CertifiedTransformation C D) := MRRTransformation.compose f g
""",
])
def test_kernel_rejects_unbound_instance_missing_domain_and_incompatible_endpoint(invalid):
    source = (cli.ROOT / "proofs/MRRProof/Transformation.lean").read_text()
    with pytest.raises(AssertionError, match="local Lean kernel rejected"):
        cli.local_lean_check(source + invalid)
