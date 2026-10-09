// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Unit reservations in two bounded lease slots; Rust owns factor/source checks.
module SearchDispatch {
  const BUG: str
  var retired: bool
  var everRetired: bool
  var active: Set[int]
  var input: int
  var output: int
  var late: bool
  action init = all { retired' = false, everRetired' = false, active' = Set(), input' = 0, output' = 0, late' = false }
  action reserve(id: int): bool = all {
    not(retired), not(active.contains(id)), input < 2 or BUG == "ignoreBudget",
    output + active.size() < 2, active' = active.union(Set(id)), input' = input + 1,
    retired' = retired, everRetired' = everRetired, output' = output, late' = late,
  }
  action admit(id: int): bool = all {
    active.contains(id), not(retired) or BUG == "admitLate",
    active' = active.exclude(Set(id)), output' = output + 1, input' = input,
    late' = (late or retired), retired' = retired, everRetired' = everRetired,
  }
  action drop(id: int): bool = all {
    active.contains(id), active' = active.exclude(Set(id)), input' = input,
    retired' = retired, everRetired' = everRetired, output' = output, late' = late,
  }
  action retire = all {
    retired' = true, everRetired' = true, active' = active, input' = input, output' = output, late' = late,
  }
  action step = any { reserve(0), reserve(1), admit(0), admit(1), drop(0), drop(1), retire }
  val safety = and { input >= 0, input <= 2, output >= 0, output + active.size() <= 2,
    active.subseteq(Set(0,1)), not(late), not(everRetired) or retired }
}
