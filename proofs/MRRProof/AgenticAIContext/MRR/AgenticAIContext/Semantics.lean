import MRR.AgenticAIContext.Tokenization

namespace MRR.AgenticAIContext

/-- Post-admission semantic element. Payloads and dependency declarations retain
their upstream meanings; this model introduces neither IDs nor temporal truth. -/
structure SemanticElement (Payload : Type) where
  payload : Payload
  dependencies : List String
  deriving DecidableEq

abbrev SemanticState (Payload : Type) := String -> SemanticElement Payload

/-- Exact element equality, including dependency declarations. Inferring this
frame from a dependency graph requires a separately complete dependency model. -/
def SemanticUnchangedOn {Payload : Type} (order : List String)
    (oldState newState : SemanticState Payload) : Prop :=
  forall name, List.Mem name order -> newState name = oldState name

def semanticRenderer {Payload : Type} (encode : SemanticElement Payload -> Bytes)
    (state : SemanticState Payload) : SegmentRenderer :=
  fun name => encode (state name)

theorem semanticStability_rendererFrame {Payload : Type}
    (order : List String) (oldState newState : SemanticState Payload)
    (encode : SemanticElement Payload -> Bytes)
    (unchanged : SemanticUnchangedOn order oldState newState) :
    RendererUnchangedOn order (semanticRenderer encode oldState) (semanticRenderer encode newState) := by
  intro name member
  exact congrArg encode (unchanged name member)

/-- Semantic equality implies identical bytes under one fixed deterministic
element encoder. No global header or implicit final-self input is permitted. -/
theorem semanticStability_bytes {Payload : Type}
    (order : List String) (oldState newState : SemanticState Payload)
    (encode : SemanticElement Payload -> Bytes)
    (unchanged : SemanticUnchangedOn order oldState newState) :
    materializeOrder (semanticRenderer encode newState) order =
      materializeOrder (semanticRenderer encode oldState) order := by
  exact materializeOrder_congr order _ _
    (semanticStability_rendererFrame order oldState newState encode unchanged)

theorem semanticLeafExtension_bytesAppend {Payload : Type}
    (order : List String) (child : String) (oldState newState : SemanticState Payload)
    (encode : SemanticElement Payload -> Bytes)
    (unchanged : SemanticUnchangedOn order oldState newState) :
    materializeOrder (semanticRenderer encode newState) (child :: order) =
      materializeOrder (semanticRenderer encode oldState) order ++ encode (newState child) := by
  exact leafExtension_bytesAppend order child _ _
    (semanticStability_rendererFrame order oldState newState encode unchanged)

theorem semanticLeafExtension_tokenPrefix {Payload : Type}
    (tokenize : Tokenizer) (order : List String) (child : String)
    (oldState newState : SemanticState Payload) (encode : SemanticElement Payload -> Bytes)
    (unchanged : SemanticUnchangedOn order oldState newState)
    (boundary : AppendComposableAt tokenize
      (materializeOrder (semanticRenderer encode oldState) order) (encode (newState child))) :
    List.IsPrefix (tokenize (materializeOrder (semanticRenderer encode oldState) order))
      (tokenize (materializeOrder (semanticRenderer encode newState) (child :: order))) := by
  exact leafExtension_tokenPrefix tokenize order child _ _
    (semanticStability_rendererFrame order oldState newState encode unchanged) boundary

end MRR.AgenticAIContext
