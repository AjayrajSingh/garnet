// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use hex;
use std::fmt;
use std::str;

use crate::errors::BlobIdParseError;

use fidl_fuchsia_pkg as fidl;

#[cfg(test)]
use proptest_derive::Arbitrary;

pub(crate) const BLOB_ID_SIZE: usize = 32;

//FIXME: wrap fidl::BlobId instead of the inner array when it implements Copy.
/// Convenience wrapper type for the autogenerated FIDL `BlobId`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BlobId([u8; BLOB_ID_SIZE]);

impl BlobId {
    /// Parse a `BlobId` from a string containing 32 lower-case hex encoded bytes.
    ///
    /// # Examples
    /// ```
    /// let s = "00112233445566778899aabbccddeeffffeeddccbbaa99887766554433221100";
    /// assert_eq!(
    ///     BlobId::parse(s),
    ///     s.parse()
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, BlobIdParseError> {
        s.parse()
    }
    /// Obtain a slice of bytes representing the `BlobId`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

impl str::FromStr for BlobId {
    type Err = BlobIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        if bytes.len() != BLOB_ID_SIZE {
            return Err(BlobIdParseError::InvalidLength(bytes.len()));
        }
        if s.chars().any(|c| c.is_uppercase()) {
            return Err(BlobIdParseError::CannotContainUppercase);
        }
        let mut res: [u8; BLOB_ID_SIZE] = [0; BLOB_ID_SIZE];
        res.copy_from_slice(&bytes[..]);
        Ok(Self(res))
    }
}

impl From<[u8; BLOB_ID_SIZE]> for BlobId {
    fn from(bytes: [u8; BLOB_ID_SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<fidl::BlobId> for BlobId {
    fn from(id: fidl::BlobId) -> Self {
        Self(id.merkle_root)
    }
}

impl Into<fidl::BlobId> for BlobId {
    fn into(self: Self) -> fidl::BlobId {
        fidl::BlobId { merkle_root: self.0 }
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

/// Convenience wrapper type for the autogenerated FIDL `BlobInfo`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlobInfo {
    pub blob_id: BlobId,
    pub length: u64,
}

impl BlobInfo {
    pub fn new(blob_id: BlobId, length: u64) -> Self {
        BlobInfo { blob_id: blob_id, length: length }
    }
}

impl From<fidl::BlobInfo> for BlobInfo {
    fn from(info: fidl::BlobInfo) -> Self {
        BlobInfo { blob_id: info.blob_id.into(), length: info.length }
    }
}

impl Into<fidl::BlobInfo> for BlobInfo {
    fn into(self: Self) -> fidl::BlobInfo {
        fidl::BlobInfo { blob_id: self.blob_id.into(), length: self.length }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn invalid_hex_char()(c in "[[:ascii:]&&[^0-9a-fA-F]]") -> char {
            assert_eq!(c.len(), 1);
            c.chars().next().unwrap()
        }
    }

    proptest! {
        #[test]
        fn parse_is_inverse_of_display(ref id in "[0-9a-f]{64}") {
            prop_assert_eq!(
                id,
                &format!("{}", id.parse::<BlobId>().unwrap())
            );
        }

        #[test]
        fn parse_is_inverse_of_debug(ref id in "[0-9a-f]{64}") {
            prop_assert_eq!(
                id,
                &format!("{:?}", id.parse::<BlobId>().unwrap())
            );
        }

        #[test]
        fn parse_rejects_uppercase(ref id in "[0-9A-F]{64}") {
            prop_assert_eq!(
                id.parse::<BlobId>(),
                Err(BlobIdParseError::CannotContainUppercase)
            );
        }

        #[test]
        fn parse_rejects_unexpected_characters(mut id in "[0-9a-f]{64}", c in invalid_hex_char(), index in 0usize..63) {
            id.remove(index);
            id.insert(index, c);
            prop_assert_eq!(
                id.parse::<BlobId>(),
                Err(BlobIdParseError::FromHexError(
                    hex::FromHexError::InvalidHexCharacter { c: c, index: index }
                ))
            );
        }

        #[test]
        fn parse_expects_even_sized_strings(ref id in "[0-9a-f]([0-9a-f]{2})*") {
            prop_assert_eq!(
                id.parse::<BlobId>(),
                Err(BlobIdParseError::FromHexError(hex::FromHexError::OddLength))
            );
        }

        #[test]
        fn parse_expects_precise_count_of_bytes(ref id in "([0-9a-f]{2})*") {
            prop_assume!(id.len() != BLOB_ID_SIZE * 2);
            prop_assert_eq!(
                id.parse::<BlobId>(),
                Err(BlobIdParseError::InvalidLength(id.len() / 2))
            );
        }

        #[test]
        fn blobid_fidl_conversions_are_inverses(id: BlobId) {
            let temp : fidl::BlobId = id.into();
            prop_assert_eq!(
                id,
                BlobId::from(temp)
            );
        }
    }
}
