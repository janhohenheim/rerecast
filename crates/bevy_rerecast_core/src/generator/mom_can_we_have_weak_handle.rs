use bevy_asset::{StrongHandle, prelude::*};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// We need this lovely construct because
/// - We want to store `Weak<StrongHandle>` as our index, since we can upgrade it to a strong handle when needed.
///   - We don't store `Handle` directly because we don't want to keep a strong reference to the asset.
///   - Keeping the asset alive is the user's choice, not the library's.
///   - We cannot store an `AssetId` instead because inserting into a dropped asset ID will cause Bevy to hit "unreachable" code lol
/// - But `Weak<StrongHandle>` is not `Eq`!!!
///   - We could use a `Vec<(A, B)>` instead of a `HashMap`, but the code in [`NavmeshGenerator::regenerate`] also wants an eq check.
///
/// So, we build this bad boi ourselves :bavy:
#[derive(Debug, Clone)]
pub(crate) struct WeHaveWeakHandleAtHome<T: Asset> {
    id: AssetId<T>,
    handle: Weak<StrongHandle>,
}

impl<T: Asset> WeHaveWeakHandleAtHome<T> {
    pub(crate) fn new(handle: &Handle<T>) -> Self {
        let id = handle.id();
        let handle = match handle {
            Handle::Strong(arc) => Arc::downgrade(arc),
            Handle::Weak(_id) => panic!("AssetIDs are not supported"),
        };
        Self { id, handle }
    }

    pub(crate) fn upgrade(&self) -> Option<Handle<T>> {
        let strong_handle = self.handle.upgrade()?;
        Some(Handle::Strong(strong_handle))
    }
}

impl<T: Asset> PartialEq for WeHaveWeakHandleAtHome<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Asset> Eq for WeHaveWeakHandleAtHome<T> {}

impl<T: Asset> Hash for WeHaveWeakHandleAtHome<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
