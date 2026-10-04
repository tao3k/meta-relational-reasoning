----------------------------- MODULE RevisionCases -----------------------------
EXTENDS RevisionLifecycle
CONSTANT Scenario
ModelOldEdges == CASE Scenario = "cycle" -> {<<0, 1>>, <<1, 0>>}
                   [] Scenario = "removed" -> {<<0, 1>>, <<1, 2>>}
                   [] Scenario = "diamond" -> {<<0, 1>>, <<0, 2>>, <<1, 3>>}
                   [] Scenario = "self" -> {<<0, 0>>}
                   [] OTHER -> {}
ModelNewEdges == CASE Scenario = "cycle" -> {<<1, 2>>, <<2, 1>>}
                   [] Scenario = "added" -> {<<0, 1>>, <<1, 2>>}
                   [] Scenario = "diamond" -> {<<2, 3>>}
                   [] OTHER -> {}
ModelUnequal == CASE Scenario = "equal" -> {}
                  [] Scenario = "diamond" -> {3}
                  [] Scenario = "self" -> {0}
                  [] Scenario = "cycle" -> {2}
                  [] OTHER -> {2}
ModelGlobal == Scenario = "global"
ModelOldSelected == IF Scenario = "selection" THEN {0, 1} ELSE {0, 1, 2, 3}
ModelNewSelected == IF Scenario = "selection" THEN {1, 2} ELSE {0, 1, 2, 3}
=============================================================================
