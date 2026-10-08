import ValueRecursive
import Lean.Util.CollectAxioms
import Lean.Elab.Command

open Lean Elab Command

run_cmd do
  let environment <- getEnv
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  let required := #[
    `MRR.ContextValueProofs.native_list_equality_exact,
    `MRR.ContextValueProofs.native_vec_equality_exact,
    `MRR.ContextValueProofs.native_entity_id_equality_exact,
    `MRR.ContextValueProofs.native_null_value_equality_exact,
    `MRR.ContextValueProofs.native_boolean_value_equality_exact,
    `MRR.ContextValueProofs.native_integer_value_equality_exact,
    `MRR.ContextValueProofs.native_decimal_value_equality_exact,
    `MRR.ContextValueProofs.native_float_value_equality_exact,
    `MRR.ContextValueProofs.native_string_value_equality_exact,
    `MRR.ContextValueProofs.native_date_value_equality_exact,
    `MRR.ContextValueProofs.native_time_value_equality_exact,
    `MRR.ContextValueProofs.native_timestamp_value_equality_exact,
    `MRR.ContextValueProofs.native_duration_value_equality_exact,
    `MRR.ContextValueProofs.native_entity_value_equality_exact,
    `MRR.ContextValueProofs.native_bytes_value_equality_exact,
    `MRR.ContextValueProofs.native_value_variant_mismatch,
    `MRR.ContextValueProofs.native_value_list_loop_short_circuit,
    `MRR.ContextValueProofs.native_value_list_loop_exhausted,
    `MRR.ContextValueProofs.native_value_list_length_mismatch,
    `MRR.ContextValueProofs.native_value_list_empty_exact,
    `MRR.ContextValueProofs.native_value_record_loop_short_circuit,
    `MRR.ContextValueProofs.native_value_record_loop_exhausted,
    `MRR.ContextValueProofs.native_value_record_length_mismatch,
    `MRR.ContextValueProofs.native_value_record_empty_exact,
    `MRR.ContextValueProofs.native_record_key_mismatch,
    `MRR.ContextValueProofs.native_record_key_match,
    `MRR.ContextValueProofs.native_list_value_delegates,
    `MRR.ContextValueProofs.native_record_value_delegates,
    `MRR.ContextValueProofs.native_atomic_value_equality_exact,
    `MRR.ContextValueProofs.native_value_list_loop_exact,
    `MRR.ContextValueProofs.native_value_record_loop_exact,
    `MRR.ContextValueProofs.native_value_lists_exact,
    `MRR.ContextValueProofs.native_value_records_exact,
    `MRR.ContextValueProofs.native_record_fields_exact,
    `MRR.ContextValueProofs.native_value_equality_total]
  required.forM fun declarationName => do
    unless environment.contains declarationName do
      throwError "Missing source theorem: {declarationName}"
    let axioms <- collectAxioms declarationName
    axioms.forM fun axiomName => do
      unless allowed.contains axiomName do
        throwError "{declarationName} depends on disallowed axiom {axiomName}"
  logInfo m!"VALUE-SOURCE-AXIOM-AUDIT-OK: {required.size} source-refinement declarations"
