import MRR.AgenticAIContext.Materialization

namespace MRR.AgenticAIContext

abbrev Tokenizer := Bytes -> TokenSequence

/-- A local boundary contract, not a property assumed of arbitrary BPE or chat
templates. Versions of renderer, template, and tokenizer must be fixed outside
this pure model. -/
def AppendComposableAt (tokenize : Tokenizer) (initial suffix : Bytes) : Prop :=
  tokenize (initial ++ suffix) = tokenize initial ++ tokenize suffix

theorem bytesAppend_tokenPrefix (tokenize : Tokenizer) (initial suffix : Bytes)
    (boundary : AppendComposableAt tokenize initial suffix) :
    List.IsPrefix (tokenize initial) (tokenize (initial ++ suffix)) := by
  exact Exists.intro (tokenize suffix) boundary.symm

/-- Token prefix stability requires the independent tokenizer boundary contract. -/
theorem leafExtension_tokenPrefix (tokenize : Tokenizer)
    (order : List String) (child : String) (oldRender newRender : SegmentRenderer)
    (unchanged : RendererUnchangedOn order oldRender newRender)
    (boundary : AppendComposableAt tokenize (materializeOrder oldRender order) (newRender child)) :
    List.IsPrefix (tokenize (materializeOrder oldRender order))
      (tokenize (materializeOrder newRender (child :: order))) := by
  rw [leafExtension_bytesAppend order child oldRender newRender unchanged]
  exact bytesAppend_tokenPrefix tokenize _ _ boundary

/-- An executable boundary-preserving reference encoding, not an LLM tokenizer. -/
def byteTokens : Tokenizer := List.map UInt8.toNat

theorem byteTokens_appendComposable (initial suffix : Bytes) :
    AppendComposableAt byteTokens initial suffix := by
  simp [AppendComposableAt, byteTokens]

end MRR.AgenticAIContext
