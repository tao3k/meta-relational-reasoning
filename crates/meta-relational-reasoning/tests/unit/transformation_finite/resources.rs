use super::TransformationError;

#[test]
fn checked_resource_composition_uses_intermediate_size_and_rejects_overflow() {
    use crate::{TransformationAffineBound as A, TransformationResourceContract as R};
    let first = R {
        size: A {
            slope: 2,
            offset: 3,
        },
        cost: A {
            slope: 1,
            offset: 2,
        },
    };
    let second = R {
        size: A {
            slope: 3,
            offset: 1,
        },
        cost: A {
            slope: 4,
            offset: 5,
        },
    };
    let combined = first.compose(second).unwrap();
    assert_eq!(combined.size.apply(7).unwrap(), 52);
    assert_eq!(combined.cost.apply(7).unwrap(), 82);
    for n in 0..100 {
        assert_eq!(
            combined.cost.apply(n).unwrap(),
            first.cost.apply(n).unwrap() + second.cost.apply(first.size.apply(n).unwrap()).unwrap()
        );
    }
    assert_eq!(
        A {
            slope: u64::MAX,
            offset: 1
        }
        .apply(1),
        Err(TransformationError::Budget)
    );
    assert_eq!(
        A {
            slope: u64::MAX,
            offset: 0
        }
        .compose(A {
            slope: 2,
            offset: 0
        }),
        Err(TransformationError::Budget)
    );
}
