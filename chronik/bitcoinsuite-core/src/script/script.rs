// Copyright (c) 2023 The Bitcoin developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

use bytes::Bytes;

use crate::{
    hash::{Hashed, ShaRmd160},
    script::{opcode::*, PubKey, ScriptMut, ScriptOpIter, UncompressedPubKey},
    ser::{BitcoinSer, BitcoinSerializer},
};

/// A Bitcoin script.
///
/// This is immutable, and uses [`Bytes`] to store the bytecode, making it cheap
/// to copy.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Script(Bytes);

impl Script {
    /// Create a new script from the given bytecode.
    pub fn new(bytecode: Bytes) -> Self {
        Script(bytecode)
    }

    /// Pay-to-public-key-hash:
    /// `OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG`
    /// ```
    /// # use bitcoinsuite_core::{script::Script, hash::ShaRmd160};
    /// # use hex_literal::hex;
    /// let hash = ShaRmd160(hex!("00112233445566778899aabbccddeeff00112233"));
    /// let script = Script::p2pkh(&hash);
    /// assert_eq!(
    ///     script.hex(),
    ///     "76a91400112233445566778899aabbccddeeff0011223388ac",
    /// );
    /// ```
    pub fn p2pkh(hash: &ShaRmd160) -> Script {
        let mut script = ScriptMut::with_capacity(2 + 1 + ShaRmd160::SIZE + 2);
        script.put_opcodes([OP_DUP, OP_HASH160]);
        script.put_bytecode(&[ShaRmd160::SIZE as u8]);
        script.put_bytecode(hash.as_le_bytes());
        script.put_opcodes([OP_EQUALVERIFY, OP_CHECKSIG]);
        script.freeze()
    }

    /// Pay-to-script-hash: `OP_HASH160 <script hash> OP_EQUAL`
    /// ```
    /// # use bitcoinsuite_core::{script::Script, hash::ShaRmd160};
    /// # use hex_literal::hex;
    /// let hash = ShaRmd160(hex!("00112233445566778899aabbccddeeff00112233"));
    /// let script = Script::p2sh(&hash);
    /// assert_eq!(
    ///     script.hex(),
    ///     "a91400112233445566778899aabbccddeeff0011223387",
    /// );
    /// ```
    pub fn p2sh(hash: &ShaRmd160) -> Script {
        let mut script = ScriptMut::with_capacity(1 + 1 + ShaRmd160::SIZE + 1);
        script.put_opcodes([OP_HASH160]);
        script.put_bytecode(&[ShaRmd160::SIZE as u8]);
        script.put_bytecode(hash.as_le_bytes());
        script.put_opcodes([OP_EQUAL]);
        script.freeze()
    }

    /// Pay-to-public-key (compressed): `<pubkey> OP_CHECKSIG`
    /// ```
    /// # use bitcoinsuite_core::{script::{PubKey, Script}, hash::ShaRmd160};
    /// # use hex_literal::hex;
    /// let pubkey = PubKey(hex!(
    ///     "0200112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
    /// ));
    /// let script = Script::p2pk(&pubkey);
    /// assert_eq!(
    ///     script.hex(),
    ///     "210200112233445566778899aabbccddeeff00112233445566778899aabbccddee\
    ///      ffac",
    /// );
    /// ```
    pub fn p2pk(pubkey: &PubKey) -> Script {
        let mut script = ScriptMut::with_capacity(1 + PubKey::SIZE + 1);
        script.put_bytecode(&[PubKey::SIZE as u8]);
        script.put_bytecode(pubkey.as_slice());
        script.put_opcodes([OP_CHECKSIG]);
        script.freeze()
    }

    /// Pay-to-public-key (uncompressed): `<pubkey> OP_CHECKSIG`
    /// ```
    /// # use bitcoinsuite_core::{
    /// #     script::{UncompressedPubKey, Script},
    /// #     hash::ShaRmd160,
    /// # };
    /// # use hex_literal::hex;
    /// let pubkey = UncompressedPubKey(hex!(
    ///     "0400112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
    ///     "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
    /// ));
    /// let script = Script::p2pk_uncompressed(&pubkey);
    /// assert_eq!(
    ///     script.hex(),
    ///     "410400112233445566778899aabbccddeeff00112233445566778899aabbccddee\
    ///      ff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\
    ///      ac",
    /// );
    /// ```
    pub fn p2pk_uncompressed(pubkey: &UncompressedPubKey) -> Script {
        let mut script =
            ScriptMut::with_capacity(1 + UncompressedPubKey::SIZE + 1);
        script.put_bytecode(&[UncompressedPubKey::SIZE as u8]);
        script.put_bytecode(pubkey.as_ref());
        script.put_opcodes([OP_CHECKSIG]);
        script.freeze()
    }

    /// Return the bytecode of the script.
    /// ```
    /// # use bitcoinsuite_core::script::Script;
    /// use bytes::Bytes;
    /// let bytecode = Bytes::from(vec![0x51]);
    /// assert_eq!(Script::new(bytecode.clone()).bytecode(), &bytecode);
    /// ```
    pub fn bytecode(&self) -> &Bytes {
        &self.0
    }

    /// Return the bytecode of the script as a [`Vec<u8>`].
    /// ```
    /// # use bitcoinsuite_core::script::Script;
    /// use bytes::Bytes;
    /// let bytecode = Bytes::from(vec![0x51]);
    /// assert_eq!(Script::new(bytecode.clone()).to_vec(), vec![0x51]);
    /// ```
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Hex of the bytecode.
    /// ```
    /// # use bitcoinsuite_core::script::Script;
    /// use bytes::Bytes;
    /// let bytecode = Bytes::from(vec![0x51]);
    /// assert_eq!(Script::new(bytecode.clone()).hex(), "51");
    /// ```
    pub fn hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Whether this script is an OP_RETURN script.
    /// ```
    /// # use bitcoinsuite_core::script::Script;
    /// assert!(Script::new(vec![0x6a].into()).is_opreturn());
    /// assert!(Script::new(vec![0x6a, 0x00].into()).is_opreturn());
    /// assert!(!Script::new(vec![0x6f].into()).is_opreturn());
    /// ```
    pub fn is_opreturn(&self) -> bool {
        match self.0.first() {
            Some(&byte) => byte == OP_RETURN.number(),
            None => false,
        }
    }

    /// Iterator over the operations in this script.
    ///
    /// ```
    /// # use bitcoinsuite_core::{
    /// #     script::{opcode::*, Op, Script},
    /// #     error::DataError,
    /// # };
    /// # use hex_literal::hex;
    /// #
    /// // Simple script
    /// let script = Script::new(hex!("0301020387").to_vec().into());
    /// let mut iter = script.iter_ops();
    /// assert_eq!(
    ///     iter.next(),
    ///     Some(Ok(Op::Push(Opcode(3), vec![1, 2, 3].into()))),
    /// );
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_EQUAL))));
    /// assert_eq!(iter.next(), None);
    ///
    /// // Complex script; has invalid op at the end
    /// let script = hex!("6a504c021234004d01001260884cffabcd");
    /// let script = Script::new(script.to_vec().into());
    /// let mut iter = script.iter_ops();
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_RETURN))));
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_RESERVED))));
    /// assert_eq!(
    ///     iter.next(),
    ///     Some(Ok(Op::Push(OP_PUSHDATA1, vec![0x12, 0x34].into()))),
    /// );
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_0))));
    /// assert_eq!(
    ///     iter.next(),
    ///     Some(Ok(Op::Push(OP_PUSHDATA2, vec![0x12].into()))),
    /// );
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_16))));
    /// assert_eq!(iter.next(), Some(Ok(Op::Code(OP_EQUALVERIFY))));
    /// assert_eq!(
    ///     iter.next(),
    ///     Some(Err(DataError::InvalidLength {
    ///         expected: 0xff,
    ///         actual: 2
    ///     })),
    /// );
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter_ops(&self) -> ScriptOpIter {
        ScriptOpIter::new(self.0.clone())
    }
}

impl AsRef<[u8]> for Script {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl BitcoinSer for Script {
    fn ser_to<S: BitcoinSerializer>(&self, bytes: &mut S) {
        self.0.ser_to(bytes)
    }
}

#[cfg(test)]
mod tests {
    use crate::{script::Script, ser::BitcoinSer};

    fn verify_ser(a: Script, b: &[u8]) {
        assert_eq!(a.ser().as_ref(), b);
        assert_eq!(a.ser_len(), b.len());
    }

    #[test]
    fn test_ser_script() {
        verify_ser(Script::default(), &[0x00]);
        verify_ser(Script::new(vec![0x51].into()), &[0x01, 0x51]);
        verify_ser(Script::new(vec![0x51, 0x52].into()), &[0x02, 0x51, 0x52]);
        verify_ser(
            Script::new(vec![4; 0xfd].into()),
            &[[0xfd, 0xfd, 0].as_ref(), &[4; 0xfd]].concat(),
        );
        verify_ser(
            Script::new(vec![5; 0x10000].into()),
            &[[0xfe, 0, 0, 1, 0].as_ref(), &vec![5; 0x10000]].concat(),
        );
    }
}
