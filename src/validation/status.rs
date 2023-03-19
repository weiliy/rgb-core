// RGB Core Library: consensus layer for RGB smart contracts.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2019-2023 Dr Maxim Orlovsky. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::iter::FromIterator;
use core::ops::AddAssign;

use bp::dbc::anchor;
use bp::{seals, Txid};

use crate::contract::Opout;
use crate::schema::{self, OpType, SchemaId};
use crate::{data, BundleId, OccurrencesMismatch, OpId, SecretSeal, StateType};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(Debug)]
#[repr(u8)]
pub enum Validity {
    Valid,
    ValidExceptEndpoints,
    UnresolvedTransactions,
    Invalid,
}

#[derive(Clone, Debug, Display, Default)]
//#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
// TODO #42: Display via YAML
#[display(Debug)]
pub struct Status {
    pub unresolved_txids: Vec<Txid>,
    pub unmined_endpoint_txids: Vec<Txid>,
    pub failures: Vec<Failure>,
    pub warnings: Vec<Warning>,
    pub info: Vec<Info>,
}

impl AddAssign for Status {
    fn add_assign(&mut self, rhs: Self) {
        self.unresolved_txids.extend(rhs.unresolved_txids);
        self.unmined_endpoint_txids
            .extend(rhs.unmined_endpoint_txids);
        self.failures.extend(rhs.failures);
        self.warnings.extend(rhs.warnings);
        self.info.extend(rhs.info);
    }
}

impl Status {
    pub fn from_error(v: Failure) -> Self {
        Status {
            unresolved_txids: vec![],
            unmined_endpoint_txids: vec![],
            failures: vec![v],
            warnings: vec![],
            info: vec![],
        }
    }
}

impl FromIterator<Failure> for Status {
    fn from_iter<T: IntoIterator<Item = Failure>>(iter: T) -> Self {
        Self {
            failures: iter.into_iter().collect(),
            ..Self::default()
        }
    }
}

impl Status {
    pub fn new() -> Self { Self::default() }

    pub fn with_failure(failure: impl Into<Failure>) -> Self {
        Self {
            failures: vec![failure.into()],
            ..Self::default()
        }
    }

    pub fn add_failure(&mut self, failure: impl Into<Failure>) -> &Self {
        self.failures.push(failure.into());
        self
    }

    pub fn add_warning(&mut self, warning: impl Into<Warning>) -> &Self {
        self.warnings.push(warning.into());
        self
    }

    pub fn add_info(&mut self, info: impl Into<Info>) -> &Self {
        self.info.push(info.into());
        self
    }

    pub fn validity(&self) -> Validity {
        if self.failures.is_empty() {
            if self.unmined_endpoint_txids.is_empty() {
                Validity::Valid
            } else {
                Validity::ValidExceptEndpoints
            }
        } else if self.unresolved_txids.is_empty() {
            Validity::Invalid
        } else {
            Validity::UnresolvedTransactions
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Display, From)]
//#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
// TODO #44: (v0.3) convert to detailed error description using doc_comments
#[display(Debug)]
pub enum Failure {
    SchemaUnknown(SchemaId),
    /// schema is a subschema, so root schema {0} must be provided for the
    /// validation
    SchemaRootRequired(SchemaId),
    /// Root schema for this schema has another root, which is prohibited
    SchemaRootHierarchy,
    SchemaRootNoFieldTypeMatch(schema::GlobalStateType),
    SchemaRootNoOwnedRightTypeMatch(schema::AssignmentType),
    SchemaRootNoPublicRightTypeMatch(schema::ValencyType),
    SchemaRootNoTransitionTypeMatch(schema::TransitionType),
    SchemaRootNoExtensionTypeMatch(schema::ExtensionType),

    SchemaRootNoMetadataMatch(OpType, schema::GlobalStateType),
    SchemaRootNoParentOwnedRightsMatch(OpType, schema::AssignmentType),
    SchemaRootNoParentPublicRightsMatch(OpType, schema::ValencyType),
    SchemaRootNoOwnedRightsMatch(OpType, schema::AssignmentType),
    SchemaRootNoPublicRightsMatch(OpType, schema::ValencyType),

    SchemaUnknownExtensionType(OpId, schema::ExtensionType),
    SchemaUnknownTransitionType(OpId, schema::TransitionType),
    SchemaUnknownFieldType(OpId, schema::GlobalStateType),
    SchemaUnknownOwnedRightType(OpId, schema::AssignmentType),
    SchemaUnknownPublicRightType(OpId, schema::ValencyType),

    SchemaDeniedScriptExtension(OpId),

    SchemaMetaValueTooSmall(schema::GlobalStateType),
    SchemaMetaValueTooLarge(schema::GlobalStateType),
    SchemaStateValueTooSmall(schema::AssignmentType),
    SchemaStateValueTooLarge(schema::AssignmentType),

    SchemaWrongEnumValue {
        field_or_state_type: u16,
        unexpected: u8,
    },
    SchemaWrongDataLength {
        field_or_state_type: u16,
        max_expected: u16,
        found: usize,
    },
    SchemaMismatchedDataType(u16),

    SchemaMetaOccurrencesError(OpId, schema::GlobalStateType, OccurrencesMismatch),
    SchemaParentOwnedRightOccurrencesError(OpId, schema::AssignmentType, OccurrencesMismatch),
    SchemaOwnedRightOccurrencesError(OpId, schema::AssignmentType, OccurrencesMismatch),

    SchemaScriptOverrideDenied,
    SchemaScriptVmChangeDenied,

    SchemaTypeSystem(/* TODO: use error from strict types */),

    // Consignment consistency errors
    OperationAbsent(OpId),
    TransitionAbsent(OpId),
    /// bundle with id {0} is invalid.
    BundleInvalid(BundleId),

    // Errors checking seal closing
    /// transition {0} is not anchored.
    NotAnchored(OpId),
    /// anchor for transition {0} doesn't commit to the actual transition data.
    NotInAnchor(OpId, Txid),
    /// transition {op} references state type {ty} absent in the outputs of
    /// previous state transition {prev_id}.
    NoPrevState {
        opid: OpId,
        prev_id: OpId,
        state_type: schema::AssignmentType,
    },
    /// transition {0} references non-existing previous output {1}.
    NoPrevOut(OpId, Opout),
    /// seal {0} present in the history is confidential and can't be validated.
    ConfidentialSeal(Opout),
    /// transition {0} is not a part of multi-protocol commitment for witness
    /// {1}; anchor is invalid.
    MpcInvalid(OpId, Txid),
    /// witness transaction {0} is not known to the transaction resolver.
    SealNoWitnessTx(Txid),
    /// transition {0} doesn't close seal with the witness transaction {1}.
    /// Details: {2}
    SealInvalid(OpId, Txid, seals::txout::VerifyError),
    /// transition {0} is not properly anchored to the witness transaction {1}.
    /// Details: {2}
    AnchorInvalid(OpId, Txid, anchor::VerifyError),

    // State extensions errors
    /// valency {valency} redeemed by state extension {opid} references
    /// non-existing operation {prev_id}
    ValencyNoParent {
        opid: OpId,
        prev_id: OpId,
        valency: schema::ValencyType,
    },
    /// state extension {opid} references valency {valency} absent in the parent
    /// {prev_id}.
    NoPrevValency {
        opid: OpId,
        prev_id: OpId,
        valency: schema::ValencyType,
    },

    // Data check errors
    /// state in {opid}/{state_type} is of {found} type, while schema requires
    /// it to be {expected}.
    StateTypeMismatch {
        opid: OpId,
        state_type: schema::AssignmentType,
        expected: StateType,
        found: StateType,
    },
    /// state in {opid}/{state_type} is of {found} type, while schema requires
    /// it to be {expected}.
    FungibleTypeMismatch {
        opid: OpId,
        state_type: schema::AssignmentType,
        expected: schema::FungibleType,
        found: schema::FungibleType,
    },
    InvalidStateDataType(OpId, u16, /* TODO: Use strict type */ data::Revealed),
    InvalidStateDataValue(OpId, u16, /* TODO: Use strict type */ Vec<u8>),
    /// invalid bulletproofs in {0}:{1}: {3}
    BulletproofsInvalid(OpId, u16, String),
    /// operation {0} is invalid: {1}
    ScriptFailure(OpId, String),

    /// Custom error by external services on top of RGB Core.
    #[display(inner)]
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Debug, Display, From)]
//#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
// TODO #44: (v0.3) convert to detailed descriptions using doc_comments
#[display(Debug)]
pub enum Warning {
    EndpointDuplication(OpId, SecretSeal),
    EndpointTransitionSealNotFound(OpId, SecretSeal),
    ExcessiveNode(OpId),
    EndpointTransactionMissed(Txid),

    /// Custom warning by external services on top of RGB Core.
    #[display(inner)]
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Debug, Display, From)]
//#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
// TODO #44: (v0.3) convert to detailed descriptions using doc_comments
#[display(Debug)]
pub enum Info {
    UncheckableConfidentialState(OpId, u16),

    /// Custom info by external services on top of RGB Core.
    #[display(inner)]
    Custom(String),
}
