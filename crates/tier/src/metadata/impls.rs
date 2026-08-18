use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::prefix::prefixed_metadata;
use super::{ConfigMetadata, FieldMetadata, TierMetadata};

impl<T> TierMetadata for crate::Secret<T> {
    fn metadata() -> ConfigMetadata {
        ConfigMetadata::from_fields([FieldMetadata::new("").secret()])
    }
}

impl TierMetadata for String {}
impl TierMetadata for bool {}
impl TierMetadata for char {}
impl TierMetadata for u8 {}
impl TierMetadata for u16 {}
impl TierMetadata for u32 {}
impl TierMetadata for u64 {}
impl TierMetadata for u128 {}
impl TierMetadata for usize {}
impl TierMetadata for i8 {}
impl TierMetadata for i16 {}
impl TierMetadata for i32 {}
impl TierMetadata for i64 {}
impl TierMetadata for i128 {}
impl TierMetadata for isize {}
impl TierMetadata for f32 {}
impl TierMetadata for f64 {}
impl TierMetadata for Duration {}
impl TierMetadata for SystemTime {}
impl TierMetadata for PathBuf {}
impl TierMetadata for IpAddr {}
impl TierMetadata for Ipv4Addr {}
impl TierMetadata for Ipv6Addr {}
impl TierMetadata for SocketAddr {}
impl TierMetadata for SocketAddrV4 {}
impl TierMetadata for SocketAddrV6 {}

impl<T> TierMetadata for Option<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        T::metadata()
    }
}

impl<T> TierMetadata for Vec<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), T::metadata())
    }
}

impl<T, const N: usize> TierMetadata for [T; N]
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), T::metadata())
    }
}

impl<T> TierMetadata for BTreeSet<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), T::metadata())
    }
}

impl<T> TierMetadata for HashSet<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), T::metadata())
    }
}

impl<K, V> TierMetadata for BTreeMap<K, V>
where
    V: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), V::metadata())
    }
}

impl<K, V, S> TierMetadata for HashMap<K, V, S>
where
    V: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        prefixed_metadata("*", Vec::new(), V::metadata())
    }
}

impl<T> TierMetadata for Box<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        T::metadata()
    }
}

impl<T> TierMetadata for Arc<T>
where
    T: TierMetadata,
{
    fn metadata() -> ConfigMetadata {
        T::metadata()
    }
}

impl IntoIterator for ConfigMetadata {
    type Item = FieldMetadata;
    type IntoIter = std::vec::IntoIter<FieldMetadata>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}
