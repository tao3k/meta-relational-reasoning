// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
//! Original native proof and physical root under a durable, terminal grant fence.
use crate::fixture;
use mrr_data_backend::{
    AuthorityExpectation, AuthorityProposal, AuthorityState, AuthorityStatus, BackendError,
    ProfilePort,
};
use mrr_data_content::{
    ConditionalCommitPortError as PortError, ConditionalContentCommitOutcome as Outcome,
    PublishReceipt,
};
use mrr_runtime::{
    SemanticRuntime,
    mrr::MrrFamilyAdmission,
    mrr_publication::{PublicationHost, PublicationPlan},
    wire::Value,
};

const SCOPE: &str = "publication-revocation";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct RetiredPublication {
    plan: PublicationPlan,
    original_guards: Vec<AuthorityExpectation>,
    retired: AuthorityState,
}

fn host<'a>(
    port: &'a ProfilePort,
    guards: &'a [AuthorityExpectation],
    grant_id: &'a str,
    admission: &'a MrrFamilyAdmission,
    runtime: &'a SemanticRuntime,
) -> PublicationHost<'a> {
    PublicationHost {
        port,
        guards,
        grant_id,
        source_id: "source",
        admission,
        runtime,
    }
}

pub async fn qualify(
    port: &ProfilePort,
    admission: &MrrFamilyAdmission,
    runtime: &SemanticRuntime,
    flow: &Value,
    physical: &PublishReceipt,
) -> Result<RetiredPublication> {
    let plan = fixture(PublicationPlan::new(
        SCOPE,
        "first",
        None,
        physical.cid,
        admission,
        flow,
    ))?;
    let source = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "source".into(),
                expected: None,
                replacement: mrr_data_content::ContentBlock::new(
                    mrr_data_content::ContentCodec::Raw,
                    admission.result()["sourceDigest"]
                        .as_str()
                        .unwrap()
                        .as_bytes(),
                )
                .cid(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    let grant = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: None,
                replacement: plan.commitment(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    let original_guards = vec![
        AuthorityExpectation {
            authority_id: "source".into(),
            state: source,
        },
        AuthorityExpectation {
            authority_id: "grant".into(),
            state: grant,
        },
    ];
    let first = match fixture(
        plan.commit(
            host(port, &original_guards, "grant", admission, runtime),
            Some(physical),
            || 2,
        )
        .await,
    )? {
        Outcome::Committed(receipt) => receipt.committed,
        Outcome::Replayed(_) => panic!("fresh revocation fixture cannot replay"),
    };
    let next = fixture(PublicationPlan::new(
        SCOPE,
        "next",
        Some(first),
        physical.cid,
        admission,
        flow,
    ))?;
    let prepared_grant = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: Some(grant),
                replacement: next.commitment(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    let stale_guards = vec![
        original_guards[0].clone(),
        AuthorityExpectation {
            authority_id: "grant".into(),
            state: prepared_grant,
        },
    ];
    // Return to identical commitment bytes while advancing the durable generation.
    let different = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: Some(prepared_grant),
                replacement: mrr_data_content::ContentBlock::new(
                    mrr_data_content::ContentCodec::Raw,
                    b"different-grant",
                )
                .cid(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    let restored = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: Some(different),
                replacement: next.commitment(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    assert_eq!(restored.commitment, prepared_grant.commitment);
    assert!(restored.generation > prepared_grant.generation);
    assert!(matches!(
        next.commit(
            host(port, &stale_guards, "grant", admission, runtime),
            Some(physical),
            || panic!("stale grant must refuse before native validation")
        )
        .await,
        Err(PortError::BeforeCommit(BackendError::AuthorityConflict))
    ));
    let current_guards = vec![
        original_guards[0].clone(),
        AuthorityExpectation {
            authority_id: "grant".into(),
            state: restored,
        },
    ];
    // The proof and flow remain currently eligible. The independent grant alone retires.
    assert_eq!(admission.current(runtime)?["status"], "current");
    assert_eq!(
        runtime.call("context.flow.evaluate", flow)?["status"],
        "eligible"
    );
    let retired = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: Some(restored),
                replacement: next.commitment(),
                status: AuthorityStatus::Retired,
            },
        )
        .await,
    )?;
    assert!(matches!(
        next.commit(
            host(port, &current_guards, "grant", admission, runtime),
            Some(physical),
            || panic!("retired grant must refuse before native validation")
        )
        .await,
        Err(PortError::BeforeCommit(BackendError::AuthorityRetired))
    ));
    assert!(matches!(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant".into(),
                expected: Some(retired),
                replacement: next.commitment(),
                status: AuthorityStatus::Active,
            }
        )
        .await,
        Err(PortError::BeforeCommit(BackendError::AuthorityRetired))
    ));
    // A second name cannot omit or erase the retired mandatory home authority.
    let alias = fixture(
        port.advance_authority(
            SCOPE,
            AuthorityProposal {
                authority_id: "grant-alias".into(),
                expected: None,
                replacement: next.commitment(),
                status: AuthorityStatus::Active,
            },
        )
        .await,
    )?;
    let alias_guards = vec![
        original_guards[0].clone(),
        AuthorityExpectation {
            authority_id: "grant".into(),
            state: retired,
        },
        AuthorityExpectation {
            authority_id: "grant-alias".into(),
            state: alias,
        },
    ];
    assert!(matches!(
        next.commit(
            host(port, &alias_guards, "grant-alias", admission, runtime),
            Some(physical),
            || panic!("alias cannot bypass retired home authority")
        )
        .await,
        Err(PortError::BeforeCommit(BackendError::AuthorityRetired))
    ));
    assert!(fixture(port.publication_delivery(SCOPE, 2).await)?.is_none());
    let delivery = fixture(port.publication_delivery(SCOPE, 1).await)?.unwrap();
    assert_eq!(delivery.committed.revision, first.revision);
    assert_eq!(delivery.committed.root, physical.cid);
    fixture(port.acknowledge_publication(&delivery).await)?;
    fixture(port.acknowledge_publication(&delivery).await)?;
    println!("NATIVE-TO-DATA-GRANT-ABA-RETIREMENT-ALIAS-REFUSED");
    Ok(RetiredPublication {
        plan,
        original_guards,
        retired,
    })
}

impl RetiredPublication {
    pub async fn verify_recovered(
        &self,
        port: &ProfilePort,
        admission: &MrrFamilyAdmission,
        runtime: &SemanticRuntime,
    ) -> Result<()> {
        assert_eq!(
            fixture(port.authority(SCOPE, "grant").await)?,
            Some(self.retired)
        );
        assert!(
            fixture(port.publication_delivery(SCOPE, 1).await)?
                .unwrap()
                .acknowledged
        );
        assert!(fixture(port.publication_delivery(SCOPE, 2).await)?.is_none());
        // This original proof is now stale. Recovery is historical and cannot revalidate or re-effect.
        assert_eq!(admission.current(runtime)?["status"], "stale");
        assert!(matches!(
            fixture(
                self.plan
                    .commit(
                        host(port, &self.original_guards, "grant", admission, runtime),
                        None,
                        || panic!("historical replay cannot read a new clock")
                    )
                    .await
            )?,
            Outcome::Replayed(_)
        ));
        println!("NATIVE-TO-DATA-RETIRED-GRANT-HISTORICAL-REPLAY-RECOVERED");
        Ok(())
    }
}
