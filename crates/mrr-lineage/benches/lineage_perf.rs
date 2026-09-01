use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_lineage::{
    FactId, LineageEdge, LineageEdgeId, LineageEdgeKind, LineageGraph, LineageNode, LineageNodeId,
    LineageNodeKind, impact, why,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

struct Fixture {
    nodes: Vec<LineageNode>,
    edges: Vec<LineageEdge>,
    graph: LineageGraph,
    source_fact: FactId,
    result: LineageNodeId,
}

fn fixture(size: usize) -> Fixture {
    let facts = (0..size)
        .map(|index| {
            FactId::from_canonical_bytes(format!("lineage:fact:{index}")).expect("fact identity")
        })
        .collect::<Vec<_>>();
    let node_ids = (0..size)
        .map(|index| {
            LineageNodeId::from_canonical_bytes(format!("lineage:node:{index}"))
                .expect("node identity")
        })
        .collect::<Vec<_>>();
    let mut nodes = Vec::with_capacity(size);
    nodes.push(LineageNode::new(
        node_ids[0],
        LineageNodeKind::SourceFact(facts[0]),
    ));
    nodes.extend(
        (1..size)
            .map(|index| LineageNode::new(node_ids[index], LineageNodeKind::Result(facts[index]))),
    );
    let edges = (1..size)
        .map(|index| {
            LineageEdge::new(
                LineageEdgeId::from_canonical_bytes(format!("lineage:edge:{index}"))
                    .expect("edge identity"),
                node_ids[index],
                node_ids[index - 1],
                LineageEdgeKind::DependsOn,
            )
        })
        .collect::<Vec<_>>();
    let graph = LineageGraph::admit(nodes.clone(), edges.clone()).expect("lineage graph");
    Fixture {
        nodes,
        edges,
        graph,
        source_fact: facts[0],
        result: node_ids[size - 1],
    }
}

fn bench(criterion: &mut Criterion) {
    for &size in SCALES {
        let fixture = fixture(size);
        let throughput = Throughput::Elements(size as u64);

        let mut why_group = criterion.benchmark_group("why_projection");
        why_group.throughput(throughput.clone());
        why_group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| why(&fixture.graph, fixture.result).expect("WHY projection"))
            },
        );
        why_group.finish();

        let mut impact_group = criterion.benchmark_group("impact_projection");
        impact_group.throughput(throughput.clone());
        impact_group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| {
                    impact(&fixture.graph, fixture.source_fact).expect("IMPACT projection")
                })
            },
        );
        impact_group.finish();

        let mut reconstruction_group = criterion.benchmark_group("lineage_reconstruction");
        reconstruction_group.throughput(throughput);
        reconstruction_group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| {
                    LineageGraph::admit(fixture.nodes.clone(), fixture.edges.clone())
                        .expect("lineage reconstruction")
                })
            },
        );
        reconstruction_group.finish();
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
