// RGB Core Library: consensus layer for RGB smart contracts.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2024 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2024 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2019-2024 Dr Maxim Orlovsky. All rights reserved.
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

use core::fmt::{self, Debug, Formatter};
use std::cmp::Ordering;

use amplify::confinement::SmallBlob;
use amplify::hex::ToHex;
use amplify::{Bytes32, Wrapper};
use bp::secp256k1::rand::{random, Rng, RngCore};
use commit_verify::{CommitId, CommitmentId, Conceal, DigestExt, Sha256};
use strict_encoding::{StrictSerialize, StrictType};

use super::{ConfidentialState, ExposedState};
use crate::{ConcealedState, RevealedState, StateType, LIB_NAME_RGB};

/// Struct using for storing Void (i.e. absent) state
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Display, Default)]
#[display("void")]
#[derive(StrictType, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
pub struct VoidState(());

impl ConfidentialState for VoidState {
    fn state_type(&self) -> StateType { StateType::Void }
    fn state_commitment(&self) -> ConcealedState { ConcealedState::Void }
}

impl ExposedState for VoidState {
    type Confidential = VoidState;
    fn state_type(&self) -> StateType { StateType::Void }
    fn state_data(&self) -> RevealedState { RevealedState::Void }
}

impl Conceal for VoidState {
    type Concealed = VoidState;
    fn conceal(&self) -> Self::Concealed { *self }
}

#[derive(Wrapper, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, From, Display, Default)]
#[display(LowerHex)]
#[wrapper(Deref, AsSlice, BorrowSlice, Hex)]
#[derive(StrictType, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB)]
pub struct DataState(SmallBlob);
impl StrictSerialize for DataState {}

impl From<RevealedData> for DataState {
    fn from(data: RevealedData) -> Self { data.value }
}

#[cfg(feature = "serde")]
mod _serde {
    use amplify::hex::FromHex;
    use serde_crate::de::Error;
    use serde_crate::{Deserialize, Deserializer, Serialize, Serializer};

    use super::*;

    impl Serialize for DataState {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for DataState {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
            let s = String::deserialize(deserializer)?;
            Self::from_hex(&s).map_err(D::Error::custom)
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB)]
#[derive(CommitEncode)]
#[commit_encode(strategy = strict, id = ConcealedData)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
pub struct RevealedData {
    pub value: DataState,
    pub salt: u128,
}

impl RevealedData {
    /// Constructs new state using the provided value using random blinding
    /// factor.
    pub fn new_random_salt(value: impl Into<DataState>) -> Self { Self::with_salt(value, random()) }

    /// Constructs new state using the provided value and random generator for
    /// creating blinding factor.
    pub fn with_rng<R: Rng + RngCore>(value: impl Into<DataState>, rng: &mut R) -> Self {
        Self::with_salt(value, rng.gen())
    }

    /// Convenience constructor.
    pub fn with_salt(value: impl Into<DataState>, salt: u128) -> Self {
        Self {
            value: value.into(),
            salt,
        }
    }
}

impl ExposedState for RevealedData {
    type Confidential = ConcealedData;
    fn state_type(&self) -> StateType { StateType::Structured }
    fn state_data(&self) -> RevealedState { RevealedState::Structured(self.clone()) }
}

impl Conceal for RevealedData {
    type Concealed = ConcealedData;

    fn conceal(&self) -> Self::Concealed { self.commit_id() }
}

impl PartialOrd for RevealedData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for RevealedData {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.value.cmp(&other.value) {
            Ordering::Equal => self.salt.cmp(&other.salt),
            other => other,
        }
    }
}

impl Debug for RevealedData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let val = String::from_utf8(self.value.to_vec()).unwrap_or_else(|_| self.value.to_hex());

        f.debug_struct("RevealedData")
            .field("value", &val)
            .field("salt", &self.salt)
            .finish()
    }
}

/// Confidential version of an structured state data.
///
/// See also revealed version [`RevealedData`].
#[derive(Wrapper, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, From)]
#[wrapper(Deref, BorrowSlice, Hex, Index, RangeOps)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB, rename = "ConcealedData")]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", transparent)
)]
pub struct ConcealedData(
    #[from]
    #[from([u8; 32])]
    Bytes32,
);

impl ConfidentialState for ConcealedData {
    fn state_type(&self) -> StateType { StateType::Structured }
    fn state_commitment(&self) -> ConcealedState { ConcealedState::Structured(*self) }
}

impl From<Sha256> for ConcealedData {
    fn from(hasher: Sha256) -> Self { hasher.finish().into() }
}

impl CommitmentId for ConcealedData {
    const TAG: &'static str = "urn:lnp-bp:rgb:state-data#2024-02-12";
}
