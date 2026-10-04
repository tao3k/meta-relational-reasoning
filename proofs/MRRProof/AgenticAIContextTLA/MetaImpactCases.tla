--------------------------- MODULE MetaImpactCases ---------------------------
EXTENDS MetaImpact
CONSTANT Scenario
ModelOldSupports ==
  CASE Scenario = "alternative" -> {<<0, 0>>, <<1, 1>>}
    [] Scenario = "last" -> {<<0, 0>>}
    [] Scenario = "incomplete" -> {<<0, 0>>}
    [] Scenario = "newEdge" -> {}
    [] OTHER -> {}
ModelNewSupports ==
  CASE Scenario = "alternative" -> {<<0, 0>>, <<1, 1>>}
    [] Scenario = "last" -> {<<0, 0>>}
    [] Scenario = "incomplete" -> {<<0, 0>>}
    [] Scenario = "newEdge" -> {<<0, 2>>}
    [] OTHER -> {}
ModelOldPresent ==
  CASE Scenario = "alternative" -> {0, 1}
    [] Scenario = "last" -> {0}
    [] Scenario = "incomplete" -> {0}
    [] OTHER -> {}
ModelNewPresent ==
  CASE Scenario = "alternative" -> {1}
    [] Scenario = "newEdge" -> {2}
    [] OTHER -> {}
ModelComplete == Scenario # "incomplete"
ModelCoverage == Scenario # "absence"
ModelOwnerClaim == IF Scenario = "temporal" THEN "contrast" ELSE "structural"
=============================================================================
