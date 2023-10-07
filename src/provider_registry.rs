use std::collections::btree_map::Entry as BTreeEntry;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Deref;

use crate::config::Section;
use crate::provider::{Provider, ProviderInfo};
use crate::CowString;

pub struct ProviderRegistry {
    /// Map of registered providers.
    /// `BTreeMap` is used to have nice alphabetic order when printing help text
    providers: BTreeMap<CowString, Box<dyn ProviderFactory>>,
}

impl Deref for ProviderRegistry {
    type Target = BTreeMap<CowString, Box<dyn ProviderFactory>>;

    fn deref(&self) -> &Self::Target {
        &self.providers
    }
}

impl ProviderRegistry {
    /// Create new provider selector
    ///
    /// # Returns
    /// New empty `Selector`
    pub fn new() -> Self {
        Self {
            providers: BTreeMap::new(),
        }
    }
    /// Adds new named provider to selector's registry
    ///
    /// # Generics
    /// * `T` - provider type to register
    ///
    /// # Parameters
    /// * `name` - provider's name
    ///
    /// # Panics
    /// If provider with such name is already registered since it's a clear programmer's error,
    /// not user's one
    pub fn add_provider<T: Provider + 'static>(&mut self, name: impl Into<CowString>) {
        let name: CowString = name.into();
        match self.providers.entry(name) {
            BTreeEntry::Vacant(e) => {
                e.insert(Box::new(ProviderFactoryT::<T>::new()));
            }
            BTreeEntry::Occupied(e) => panic!("Provider {} already registered", e.key()),
        }
    }
}
/// Factory wrapper for any weather provider
/// Required to virtualize static methods of specific `Provider` implementor
pub trait ProviderFactory {
    /// Delegates to `Provider::new`, which in turn returns boxed provider instance
    ///
    /// # Parameters
    /// * `config` - TOML config for this provider, concrete format depends on implementor
    ///
    /// # Returns
    /// Boxed future which completes with boxed provider instance or error
    fn create(&self, config: &Section) -> anyhow::Result<Box<dyn Provider>>;
    /// Get additional information about provider
    ///
    /// # Returns
    /// Provider information
    fn info(&self) -> &'static ProviderInfo;
}
/// Factory companion to type which implements `Provider` trait
///
/// Doesn't store any data itself, just provides dynamic dispatch for `T`'s static methods
///
/// # Generics
/// * `T` - actual type which implements
struct ProviderFactoryT<T: Provider + 'static>(PhantomData<T>);

impl<T: Provider + 'static> ProviderFactoryT<T> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: Provider + 'static> ProviderFactory for ProviderFactoryT<T> {
    fn create(&self, config: &Section) -> anyhow::Result<Box<dyn Provider>> {
        T::new(config).map(|p| Box::new(p) as Box<dyn Provider>)
    }

    fn info(&self) -> &'static ProviderInfo {
        T::info()
    }
}
