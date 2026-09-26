//! Differential qualification for POO Flow's positive binary rule fragment.
//! This test invokes Scheme only as an external proposal engine; it does not
//! admit MRR lineage, authenticate a source revision, or extend bundle rules.

use ascent::ascent;
use std::{
    collections::BTreeSet,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Debug, Eq, PartialEq)]
struct GuardedRows {
    selected: BTreeSet<(u32, u32)>,
    copied: BTreeSet<(u32, u32)>,
    twohop: BTreeSet<(u32, u32)>,
}

fn ascent_rows(edges: &[(u32, u32)]) -> GuardedRows {
    ascent! {
        relation edge(u32, u32);
        relation selected(u32, u32);
        relation copied(u32, u32);
        relation twohop(u32, u32);

        selected(from, to) <-- edge(from, to), if from % 2 == 0;
        copied(from, to) <-- selected(from, to);
        twohop(from, to) <-- selected(from, via), edge(via, to);
    }

    let mut program = AscentProgram {
        edge: edges.to_vec(),
        ..AscentProgram::default()
    };
    program.run();
    GuardedRows {
        selected: program.selected.into_iter().collect(),
        copied: program.copied.into_iter().collect(),
        twohop: program.twohop.into_iter().collect(),
    }
}

fn scheme_rows(edges: &[(u32, u32)]) -> GuardedRows {
    let root =
        PathBuf::from(std::env::var_os("MRR_POO_FLOW_ROOT").expect("POO Flow checkout path"));
    let mut request = String::from("(8");
    for &(from, to) in edges {
        assert!(from < 8 && to < 8);
        request.push_str(&format!(" {from} {to}"));
    }
    request.push_str(")\n");
    let mut command = Command::new("just");
    command
        .current_dir(&root)
        .arg("ascent-guarded")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut loadpath = format!("{}:{}", root.display(), root.join(".gerbil/lib").display());
    if let Some(extra) = std::env::var_os("MRR_POO_FLOW_EXTRA_LOADPATH") {
        loadpath.push(':');
        loadpath.push_str(&extra.to_string_lossy());
    }
    command.env("GERBIL_PATH", root.join(".gerbil"));
    command.env("GERBIL_LOADPATH", loadpath);
    let mut child = command.spawn().expect("launch POO Flow guarded evaluator");
    child
        .stdin
        .take()
        .expect("guarded fixture stdin")
        .write_all(request.as_bytes())
        .expect("write source edges to Scheme evaluator");
    let output = child.wait_with_output().expect("collect guarded rows");
    assert!(
        output.status.success(),
        "Scheme evaluator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut rows = GuardedRows {
        selected: BTreeSet::new(),
        copied: BTreeSet::new(),
        twohop: BTreeSet::new(),
    };
    let stdout = String::from_utf8(output.stdout).expect("Scheme output is UTF-8");
    for line in stdout.lines() {
        let mut fields = line.split('\t');
        let relation = fields.next().expect("relation name");
        let from = fields
            .next()
            .expect("from node")
            .parse()
            .expect("numeric from");
        let to = fields.next().expect("to node").parse().expect("numeric to");
        assert!(fields.next().is_none(), "unexpected Scheme output field");
        let target = match relation {
            "selected" => &mut rows.selected,
            "copied" => &mut rows.copied,
            "twohop" => &mut rows.twohop,
            other => panic!("unknown Scheme relation: {other}"),
        };
        assert!(target.insert((from, to)), "duplicate Scheme output row");
    }
    rows
}

#[test]
#[ignore = "requires a built POO Flow checkout in MRR_POO_FLOW_ROOT"]
fn live_scheme_guarded_copy_join_matches_ascent_for_source_snapshots() {
    let snapshots: &[&[(u32, u32)]] = &[
        &[(1, 2), (2, 3), (2, 4), (4, 5), (3, 5)],
        &[(1, 2), (2, 3), (2, 4), (3, 5)],
        &[(1, 2), (2, 3), (2, 4)],
        &[(1, 2), (3, 4)],
        &[(2, 2), (2, 3), (3, 4)],
    ];
    for (index, edges) in snapshots.iter().enumerate() {
        assert_eq!(
            scheme_rows(edges),
            ascent_rows(edges),
            "guarded filter/copy/join mismatch at source snapshot {index}"
        );
    }
}
