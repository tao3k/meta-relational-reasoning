import TransformGenerated.Funs

open Aeneas.Std
open scoped Aeneas
open MRR.TransformationRust

namespace MRR.TransformationTableProofs

/-- Actual extracted forward returns precisely the selected table cell, including
out-of-domain rejection. Slice access is the pinned Aeneas library model. -/
theorem forward_exact (table : Slice Usize) (input : Usize) :
    transformation_table.forward table input = .ok table[input]? := by
  cases h : table[input]? <;>
    simp [transformation_table.forward, core.slice.Slice.get,
      h]

/-- Actual extracted solver selects the first allowed answer. Empty rows and
missing instances have no answer; they are never silently mapped to zero. -/
theorem solve_exact (table : Slice (alloc.vec.Vec Usize)) (input : Usize) :
    transformation_table.solve table input =
      .ok ((table[input]?).bind fun row => (alloc.vec.Vec.deref row)[0#usize]?) := by
  cases h : table[input]? <;>
    simp [transformation_table.solve, core.slice.Slice.get,
      forward_exact, h]

/-- Actual extracted extractor uses the original source instance and actual
target answer as its two coordinates. -/
theorem extract_exact (table : Slice (alloc.vec.Vec Usize))
    (input answer : Usize) :
    transformation_table.extract table input answer =
      .ok ((table[input]?).bind fun row => (alloc.vec.Vec.deref row)[answer]?) := by
  cases h : table[input]? <;>
    simp [transformation_table.extract, core.slice.Slice.get,
      forward_exact, h]

theorem solve_is_zero_answer_extraction
    (table : Slice (alloc.vec.Vec Usize)) (input : Usize) :
    transformation_table.solve table input =
      transformation_table.extract table input 0#usize := by
  rw [solve_exact, extract_exact]

theorem missing_forward (table : Slice Usize) (input : Usize)
    (missing : table[input]? = none) :
    transformation_table.forward table input = .ok none := by
  rw [forward_exact, missing]

theorem missing_source_extraction (table : Slice (alloc.vec.Vec Usize))
    (input answer : Usize) (missing : table[input]? = none) :
    transformation_table.extract table input answer = .ok none := by
  rw [extract_exact, missing]
  rfl

/-- The production byte clone succeeds with precisely the original sequence.
Allocator behavior and the pinned Aeneas Slice/Vec models remain trust boundaries. -/
theorem transport_bytes_exact (bytes : Slice U8) :
    transformation_table.transport_bytes bytes ⦃ copied => bytes = copied.slice ⦄ := by
  simpa [transformation_table.transport_bytes] using
    (alloc.slice.Slice.to_vec_spec core.clone.CloneU8 bytes (by intro x hx; rfl))

end MRR.TransformationTableProofs
