// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Property resolution against exact entity and relation catalogs.

use mrr_bundle::ReasoningBundle;
use mrr_query::{Binding, PropertyKey};
use mrr_relation::RelationField;

use crate::QueryCatalogBindingError;

use super::checker::BindingTarget;

pub(super) fn resolve_property<'a>(
    target: &BindingTarget,
    binding: &Binding,
    key: &PropertyKey,
    bundle: &'a ReasoningBundle,
) -> Result<&'a RelationField, QueryCatalogBindingError> {
    let mut fields = Vec::new();
    match target {
        BindingTarget::Node(types) => {
            if types.is_empty() {
                return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                    binding.clone(),
                ));
            }
            for entity in types {
                let schema = bundle
                    .entity_catalog()
                    .entity(*entity)
                    .ok_or(QueryCatalogBindingError::UnknownEntity(*entity))?;
                fields.push(schema.property(key.as_str()).ok_or_else(|| {
                    QueryCatalogBindingError::UnknownProperty {
                        binding: binding.clone(),
                        key: key.clone(),
                    }
                })?);
            }
        }
        BindingTarget::Relation(types) => {
            if types.is_empty() {
                return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                    binding.clone(),
                ));
            }
            for relation in types {
                let schema = bundle
                    .relation_catalog()
                    .relation(*relation)
                    .ok_or(QueryCatalogBindingError::UnknownRelation(*relation))?;
                fields.push(
                    schema
                        .fields()
                        .iter()
                        .find(|field| field.name() == key.as_str())
                        .ok_or_else(|| QueryCatalogBindingError::UnknownProperty {
                            binding: binding.clone(),
                            key: key.clone(),
                        })?,
                );
            }
        }
        BindingTarget::Scalar(_) => {
            return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                binding.clone(),
            ));
        }
    }
    let first = fields[0];
    if fields
        .iter()
        .skip(1)
        .any(|field| field.schema() != first.schema() || field.nullable() != first.nullable())
    {
        return Err(QueryCatalogBindingError::ConflictingPropertySchema {
            binding: binding.clone(),
            key: key.clone(),
        });
    }
    Ok(first)
}
