#[test]
fn parser_owned_cypher_uses_its_exact_native_grammar() {
    let source = "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.age > 18 RETURN f.name\n";
    let artifact = crate::parse_cypher_artifact(source).expect("parser-owned Cypher artifact");
    assert_eq!(artifact.language, crate::ParserLanguage::Cypher);
    assert_eq!(
        artifact.kind_catalog.language(),
        crate::ParserLanguage::Cypher
    );
    assert_eq!(artifact.status, crate::ParseArtifactStatus::Accepted);
    assert_eq!(artifact.kind_catalog.kind_id("program"), Some(0));
    assert!(matches!(
        artifact.events.first(),
        Some(crate::ParseEvent::StartNode { kind, start: 0, .. }) if kind == "program"
    ));
    let cst = artifact
        .to_rowan_cst()
        .expect("accepted Cypher artifact must sink into Rowan");
    assert_eq!(cst.root().text().to_string(), source);
}

#[test]
fn rowan_sink_rejects_malformed_parser_event_structure() {
    let source = "MATCH (n) RETURN n\n";
    let artifact = crate::parse_gql_artifact(source).expect("valid parser artifact");

    let mut wrong_node = artifact.clone();
    let finish = wrong_node
        .events
        .iter_mut()
        .find(|event| matches!(event, crate::ParseEvent::FinishNode { .. }))
        .expect("finish node");
    if let crate::ParseEvent::FinishNode { id, .. } = finish {
        *id = u64::MAX;
    }
    assert!(matches!(
        wrong_node.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidEventOrder { .. })
    ));

    let mut wrong_offset = artifact.clone();
    let token = wrong_offset
        .events
        .iter_mut()
        .find(|event| matches!(event, crate::ParseEvent::Token { .. }))
        .expect("token event");
    if let crate::ParseEvent::Token { start, .. } = token {
        *start = 1;
    }
    assert!(matches!(
        wrong_offset.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidOffset { .. })
    ));

    let mut unknown_kind = artifact;
    let node = unknown_kind
        .events
        .iter_mut()
        .find(|event| matches!(event, crate::ParseEvent::StartNode { .. }))
        .expect("start node");
    if let crate::ParseEvent::StartNode { kind, .. } = node {
        *kind = "RustInventedNode".to_owned();
    }
    assert!(matches!(
        unknown_kind.to_rowan_cst(),
        Err(crate::ParserCstError::UnknownNodeKind { .. })
    ));
}

#[test]
fn rowan_sink_rejects_events_outside_one_balanced_root() {
    let source = "MATCH (n) RETURN n\n";
    let artifact = crate::parse_gql_artifact(source).expect("valid parser artifact");

    let mut empty = artifact.clone();
    empty.events.clear();
    assert_eq!(
        empty.to_rowan_cst(),
        Err(crate::ParserCstError::MissingRoot)
    );

    let mut multiple_roots = artifact.clone();
    multiple_roots.events.extend(artifact.events.clone());
    assert!(matches!(
        multiple_roots.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidEventOrder { .. })
    ));

    let mut unclosed_node = artifact.clone();
    unclosed_node.events.pop();
    assert_eq!(
        unclosed_node.to_rowan_cst(),
        Err(crate::ParserCstError::MissingRoot)
    );

    let mut outside_field = artifact.clone();
    outside_field.events.insert(
        0,
        crate::ParseEvent::StartField {
            field: "outside".to_owned(),
            start: 0,
        },
    );
    assert!(matches!(
        outside_field.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidEventOrder { .. })
    ));

    let mut unclosed_field = artifact.clone();
    unclosed_field.events.insert(
        1,
        crate::ParseEvent::StartField {
            field: "open".to_owned(),
            start: 0,
        },
    );
    assert!(matches!(
        unclosed_field.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidEventOrder { .. })
    ));

    let mut outside_token = artifact.clone();
    let end = source.len() as u32;
    outside_token.events.push(crate::ParseEvent::Token {
        id: u64::MAX,
        kind: "unknown".to_owned(),
        lexeme: String::new(),
        start: end,
        end,
    });
    assert!(matches!(
        outside_token.to_rowan_cst(),
        Err(crate::ParserCstError::InvalidEventOrder { .. })
    ));
}

#[test]
fn rowan_sink_rejects_unknown_or_non_node_catalog_kinds() {
    let source = "MATCH (n) RETURN n\n";
    let artifact = crate::parse_gql_artifact(source).expect("valid parser artifact");

    let mut unknown_token = artifact.clone();
    let token = unknown_token
        .events
        .iter_mut()
        .find(|event| matches!(event, crate::ParseEvent::Token { .. }))
        .expect("token event");
    if let crate::ParseEvent::Token { kind, .. } = token {
        *kind = "not-a-terminal".to_owned();
    }
    assert!(matches!(
        unknown_token.to_rowan_cst(),
        Err(crate::ParserCstError::UnknownTokenKind { .. })
    ));

    let mut token_as_node = artifact;
    let root = token_as_node
        .events
        .iter_mut()
        .find(|event| matches!(event, crate::ParseEvent::StartNode { .. }))
        .expect("root event");
    if let crate::ParseEvent::StartNode { kind, .. } = root {
        *kind = "UnknownToken".to_owned();
    }
    assert!(matches!(
        token_as_node.to_rowan_cst(),
        Err(crate::ParserCstError::KindCategoryMismatch { .. })
    ));
}
