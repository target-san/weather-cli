use std::borrow::Cow;
use std::collections::btree_map::Entry as BTreeEntry;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use crate::provider::Provider;

pub struct ProviderRegistry {
    /// Map of registered providers.
    /// `BTreeMap` is used to have nice alphabetic order when printing help text
    providers: BTreeMap<Cow<'static, str>, Box<dyn ProviderFactory>>,
}

impl Deref for ProviderRegistry {
    type Target = BTreeMap<Cow<'static, str>, Box<dyn ProviderFactory>>;

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
    pub fn add_provider<T: Provider + 'static>(&mut self, name: impl Into<Cow<'static, str>>) {
        let name: Cow<'static, str> = name.into();
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
    /// Delegates to `Provider::help`,
    /// which in turn outputs details on concrete provider into formatter
    ///
    /// # Parameters
    /// * `f` - formatter
    ///
    /// # Returns
    /// Formatting result
    fn help(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    /// Delegates to `Provider::new`, which in turn returns boxed provider instance
    ///
    /// # Parameters
    /// * `config` - TOML config for this provider, concrete format depends on implementor
    ///
    /// # Returns
    /// Boxed future which completes with boxed provider instance or error
    fn create(&self, config: toml::Value) -> anyhow::Result<Box<dyn Provider>>;
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
    fn help(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::help(f)
    }

    fn create(&self, config: toml::Value) -> anyhow::Result<Box<dyn Provider>> {
        T::new(config).map(|p| Box::new(p) as Box<dyn Provider>)
    }
}
