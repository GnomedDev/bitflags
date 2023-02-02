use core::{fmt, str};
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

pub fn serialize_bits_default<T: fmt::Display + AsRef<B>, B: Serialize, S: Serializer>(
    flags: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.collect_str(flags)
    } else {
        flags.as_ref().serialize(serializer)
    }
}

pub fn deserialize_bits_default<
    'de,
    T: str::FromStr + From<B>,
    B: Deserialize<'de>,
    D: Deserializer<'de>,
>(
    deserializer: D,
) -> Result<T, D::Error>
where
    <T as str::FromStr>::Err: fmt::Display,
{
    if deserializer.is_human_readable() {
        struct FlagsVisitor<T>(core::marker::PhantomData<T>);

        impl<'de, T: str::FromStr> Visitor<'de> for FlagsVisitor<T>
        where
            <T as str::FromStr>::Err: fmt::Display,
        {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string value of `|` separated flags")
            }

            fn visit_str<E: Error>(self, flags: &str) -> Result<Self::Value, E> {
                flags.parse().map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(FlagsVisitor(Default::default()))
    } else {
        let bits = B::deserialize(deserializer)?;

        Ok(bits.into())
    }
}

pub mod legacy_format {
    //! Generic implementations of `serde::Serialize` and `serde::Deserialize` for flags types
    //! that's compatible with `#[derive(Serialize, Deserialize)]` on types generated by
    //! `bitflags` `1.x`.
    //!
    //! # Using this module
    //!
    //! When upgrading from `bitflags` `1.x`, replace your `#[derive(Serialize, Deserialize)]`
    //! with the following manual implementations:
    //!
    //!
    //! ```
    //! bitflags! {
    //!     // #[derive(Serialize, Deserialize)]
    //!     struct SerdeLegacyFlags: u32 {
    //!         const A = 1;
    //!         const B = 2;
    //!         const C = 4;
    //!         const D = 8;
    //!     }
    //! }
    //!
    //! impl serde::Serialize for SerdeLegacyFlags {
    //!     fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    //!         bitflags::serde_support::legacy_format::serialize(self, serializer)
    //!     }
    //! }
    //!
    //! impl<'de> serde::Deserialize<'de> for SerdeLegacyFlags {
    //!     fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    //!         bitflags::serde_support::legacy_format::deserialize(deserializer)
    //!     }
    //! }
    //! ```

    use core::{fmt, any::type_name};
    use serde::{Serialize, Serializer, Deserialize, Deserializer, ser::SerializeStruct, de::{Error, Visitor, MapAccess}};

    use crate::BitFlags;

    /// Serialize a flags type equivalently to how `#[derive(Serialize)]` on a flags type
    /// from `bitflags` `1.x` would.
    pub fn serialize<T: BitFlags, S: Serializer>(flags: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        <T as BitFlags>::Bits: Serialize,
    {
        let mut serialize_struct = serializer.serialize_struct(type_name::<T>(), 1)?;
        serialize_struct.serialize_field("bits", &flags.bits())?;
        serialize_struct.end()
    }

    /// Deserialize a flags type equivalently to how `#[derive(Deserialize)]` on a flags type
    /// from `bitflags` `1.x` would.
    pub fn deserialize<'de, T: BitFlags, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error>
    where
        <T as BitFlags>::Bits: Deserialize<'de>,
    {
        struct BitsVisitor<T>(core::marker::PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for BitsVisitor<T> {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a primitive bitflags value wrapped in a struct")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut bits = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "bits" => {
                            if bits.is_some() {
                                return Err(Error::duplicate_field("bits"));
                            }

                            bits = Some(map.next_value()?);
                        }
                        v => return Err(Error::unknown_field(v, &["bits"])),
                    }
                }

                bits.ok_or_else(|| Error::missing_field("bits"))
            }
        }

        let bits = deserializer.deserialize_struct(type_name::<T>(), &["bits"], BitsVisitor(Default::default()))?;

        Ok(T::from_bits_retain(bits))
    }
}

#[cfg(test)]
mod tests {
    bitflags! {
        #[derive(serde_derive::Serialize, serde_derive::Deserialize)]
        struct SerdeFlags: u32 {
            const A = 1;
            const B = 2;
            const C = 4;
            const D = 8;
        }
    }

    bitflags! {
        struct SerdeLegacyFlags: u32 {
            const A = 1;
            const B = 2;
            const C = 4;
            const D = 8;
        }
    }

    impl serde::Serialize for SerdeLegacyFlags {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            crate::serde_support::legacy_format::serialize(self, serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for SerdeLegacyFlags {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            crate::serde_support::legacy_format::deserialize(deserializer)
        }
    }

    #[test]
    fn test_serde_bitflags_default_serialize() {
        let flags = SerdeFlags::A | SerdeFlags::B;

        let serialized = serde_json::to_string(&flags).unwrap();

        assert_eq!(serialized, r#""A | B""#);
    }

    #[test]
    fn test_serde_bitflags_default_deserialize() {
        let deserialized: SerdeFlags = serde_json::from_str(r#""C | D""#).unwrap();

        let expected = SerdeFlags::C | SerdeFlags::D;

        assert_eq!(deserialized.bits(), expected.bits());
    }

    #[test]
    fn test_serde_bitflags_default_roundtrip() {
        let flags = SerdeFlags::A | SerdeFlags::B;

        let deserialized: SerdeFlags =
            serde_json::from_str(&serde_json::to_string(&flags).unwrap()).unwrap();

        assert_eq!(deserialized.bits(), flags.bits());
    }

    #[test]
    fn test_serde_bitflags_legacy_serialize() {
        let flags = SerdeLegacyFlags::A | SerdeLegacyFlags::B;

        let serialized = serde_json::to_string(&flags).unwrap();

        assert_eq!(serialized, r#"{"bits":3}"#);
    }

    #[test]
    fn test_serde_bitflags_legacy_deserialize() {
        let deserialized: SerdeLegacyFlags = serde_json::from_str(r#"{"bits":12}"#).unwrap();

        let expected = SerdeLegacyFlags::C | SerdeLegacyFlags::D;

        assert_eq!(deserialized.bits(), expected.bits());
    }

    #[test]
    fn test_serde_bitflags_legacy_roundtrip() {
        let flags = SerdeLegacyFlags::A | SerdeLegacyFlags::B;

        let deserialized: SerdeLegacyFlags =
            serde_json::from_str(&serde_json::to_string(&flags).unwrap()).unwrap();

        assert_eq!(deserialized.bits(), flags.bits());
    }
}
