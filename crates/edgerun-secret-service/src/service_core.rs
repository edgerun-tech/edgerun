//! Transport-free secret service request dispatcher.
//!
//! This layer owns the typed secret-service request/response contract. It does
//! not own D-Bus, HTTP, mesh routing, filesystem paths, BlobStore, or FileIndex.

use crate::prelude::v1::*;
use alloc::collections::BTreeMap as HashMap;
use std::io;

use crate::backend::{Backend, CredentialMeta};

#[derive(Clone, Debug)]
pub enum SecretRequest {
    Put {
        collection: String,
        key: String,
        secret: Vec<u8>,
        label: String,
        attributes: Vec<(String, String)>,
    },
    Get {
        collection: String,
        key: String,
    },
    Delete {
        collection: String,
        key: String,
    },
    List {
        collection: String,
    },
    Search {
        collection: String,
        attributes: Vec<(String, String)>,
    },
    ListCollections,
    CollectionExists {
        collection: String,
    },
    CreateCollection {
        collection_name: String,
        label: String,
    },
    DeleteCollection {
        collection_name: String,
    },
}

#[derive(Clone, Debug)]
pub enum SecretResponse {
    Unit,
    Secret(Option<SecretEntry>),
    Deleted(bool),
    Items(Vec<(String, CredentialMeta)>),
    Collections(Vec<String>),
    Exists(bool),
    DeletedCollection { items_removed: u32 },
}

#[derive(Clone, Debug)]
pub struct SecretEntry {
    pub secret: Vec<u8>,
    pub meta: CredentialMeta,
}

pub trait SecretStore {
    fn put_secret(
        &mut self,
        collection: &str,
        key: &str,
        secret: &[u8],
        label: &str,
        attributes: &[(String, String)],
    ) -> io::Result<()>;

    fn get_secret(
        &self,
        collection: &str,
        key: &str,
    ) -> io::Result<Option<(Vec<u8>, CredentialMeta)>>;

    fn delete_secret(&mut self, collection: &str, key: &str) -> io::Result<bool>;

    fn list_secrets(&self, collection: &str) -> io::Result<Vec<(String, CredentialMeta)>>;

    fn search_secrets(
        &self,
        collection: &str,
        attributes: &[(String, String)],
    ) -> io::Result<Vec<(String, CredentialMeta)>>;

    fn list_secret_collections(&self) -> io::Result<Vec<String>>;

    fn secret_collection_exists(&self, collection: &str) -> bool;

    fn create_secret_collection(&mut self, collection_name: &str, label: &str) -> io::Result<()>;

    fn delete_secret_collection(&mut self, collection_name: &str) -> io::Result<u32>;
}

impl<T: SecretStore + ?Sized> SecretStore for &mut T {
    fn put_secret(
        &mut self,
        collection: &str,
        key: &str,
        secret: &[u8],
        label: &str,
        attributes: &[(String, String)],
    ) -> io::Result<()> {
        (**self).put_secret(collection, key, secret, label, attributes)
    }

    fn get_secret(
        &self,
        collection: &str,
        key: &str,
    ) -> io::Result<Option<(Vec<u8>, CredentialMeta)>> {
        (**self).get_secret(collection, key)
    }

    fn delete_secret(&mut self, collection: &str, key: &str) -> io::Result<bool> {
        (**self).delete_secret(collection, key)
    }

    fn list_secrets(&self, collection: &str) -> io::Result<Vec<(String, CredentialMeta)>> {
        (**self).list_secrets(collection)
    }

    fn search_secrets(
        &self,
        collection: &str,
        attributes: &[(String, String)],
    ) -> io::Result<Vec<(String, CredentialMeta)>> {
        (**self).search_secrets(collection, attributes)
    }

    fn list_secret_collections(&self) -> io::Result<Vec<String>> {
        (**self).list_secret_collections()
    }

    fn secret_collection_exists(&self, collection: &str) -> bool {
        (**self).secret_collection_exists(collection)
    }

    fn create_secret_collection(&mut self, collection_name: &str, label: &str) -> io::Result<()> {
        (**self).create_secret_collection(collection_name, label)
    }

    fn delete_secret_collection(&mut self, collection_name: &str) -> io::Result<u32> {
        (**self).delete_secret_collection(collection_name)
    }
}

pub struct SecretServiceCore<S> {
    store: S,
}

impl<S: SecretStore> SecretServiceCore<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    pub fn into_store(self) -> S {
        self.store
    }

    pub fn dispatch(&mut self, request: SecretRequest) -> io::Result<SecretResponse> {
        match request {
            SecretRequest::Put {
                collection,
                key,
                secret,
                label,
                attributes,
            } => {
                self.store
                    .put_secret(&collection, &key, &secret, &label, &attributes)?;
                Ok(SecretResponse::Unit)
            }
            SecretRequest::Get { collection, key } => {
                let entry = self
                    .store
                    .get_secret(&collection, &key)?
                    .map(|(secret, meta)| SecretEntry { secret, meta });
                Ok(SecretResponse::Secret(entry))
            }
            SecretRequest::Delete { collection, key } => {
                let deleted = self.store.delete_secret(&collection, &key)?;
                Ok(SecretResponse::Deleted(deleted))
            }
            SecretRequest::List { collection } => {
                let items = self.store.list_secrets(&collection)?;
                Ok(SecretResponse::Items(items))
            }
            SecretRequest::Search {
                collection,
                attributes,
            } => {
                let items = self.store.search_secrets(&collection, &attributes)?;
                Ok(SecretResponse::Items(items))
            }
            SecretRequest::ListCollections => {
                let collections = self.store.list_secret_collections()?;
                Ok(SecretResponse::Collections(collections))
            }
            SecretRequest::CollectionExists { collection } => Ok(SecretResponse::Exists(
                self.store.secret_collection_exists(&collection),
            )),
            SecretRequest::CreateCollection {
                collection_name,
                label,
            } => {
                self.store
                    .create_secret_collection(&collection_name, &label)?;
                Ok(SecretResponse::Unit)
            }
            SecretRequest::DeleteCollection { collection_name } => {
                let items_removed = self.store.delete_secret_collection(&collection_name)?;
                Ok(SecretResponse::DeletedCollection { items_removed })
            }
        }
    }
}

impl SecretStore for Backend {
    fn put_secret(
        &mut self,
        collection: &str,
        key: &str,
        secret: &[u8],
        label: &str,
        attributes: &[(String, String)],
    ) -> io::Result<()> {
        self.put(collection, key, secret, label, attributes)
    }

    fn get_secret(
        &self,
        collection: &str,
        key: &str,
    ) -> io::Result<Option<(Vec<u8>, CredentialMeta)>> {
        self.get(collection, key)
    }

    fn delete_secret(&mut self, collection: &str, key: &str) -> io::Result<bool> {
        self.delete(collection, key)
    }

    fn list_secrets(&self, collection: &str) -> io::Result<Vec<(String, CredentialMeta)>> {
        self.list(collection)
    }

    fn search_secrets(
        &self,
        collection: &str,
        attributes: &[(String, String)],
    ) -> io::Result<Vec<(String, CredentialMeta)>> {
        self.search(collection, attributes)
    }

    fn list_secret_collections(&self) -> io::Result<Vec<String>> {
        self.list_collections()
    }

    fn secret_collection_exists(&self, collection: &str) -> bool {
        self.collection_exists(collection)
    }

    fn create_secret_collection(&mut self, collection_name: &str, label: &str) -> io::Result<()> {
        self.create_collection(collection_name, label)
    }

    fn delete_secret_collection(&mut self, collection_name: &str) -> io::Result<u32> {
        self.delete_collection(collection_name)
    }
}

#[derive(Default)]
pub struct MemorySecretStore {
    collections: HashMap<String, HashMap<String, SecretEntry>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecretStore {
    fn put_secret(
        &mut self,
        collection: &str,
        key: &str,
        secret: &[u8],
        label: &str,
        attributes: &[(String, String)],
    ) -> io::Result<()> {
        let meta = CredentialMeta {
            label: label.to_string(),
            attributes: attributes.iter().cloned().collect(),
            created_us: 0,
        };
        self.collections
            .entry(collection.to_string())
            .or_default()
            .insert(
                key.to_string(),
                SecretEntry {
                    secret: secret.to_vec(),
                    meta,
                },
            );
        Ok(())
    }

    fn get_secret(
        &self,
        collection: &str,
        key: &str,
    ) -> io::Result<Option<(Vec<u8>, CredentialMeta)>> {
        Ok(self
            .collections
            .get(collection)
            .and_then(|items| items.get(key))
            .map(|entry| (entry.secret.clone(), entry.meta.clone())))
    }

    fn delete_secret(&mut self, collection: &str, key: &str) -> io::Result<bool> {
        Ok(self
            .collections
            .get_mut(collection)
            .and_then(|items| items.remove(key))
            .is_some())
    }

    fn list_secrets(&self, collection: &str) -> io::Result<Vec<(String, CredentialMeta)>> {
        Ok(self
            .collections
            .get(collection)
            .map(|items| {
                items
                    .iter()
                    .map(|(key, entry)| (key.clone(), entry.meta.clone()))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn search_secrets(
        &self,
        collection: &str,
        attributes: &[(String, String)],
    ) -> io::Result<Vec<(String, CredentialMeta)>> {
        let items = self.list_secrets(collection)?;
        if attributes.is_empty() {
            return Ok(items);
        }
        Ok(items
            .into_iter()
            .filter(|(_, meta)| {
                attributes
                    .iter()
                    .all(|(k, v)| meta.attributes.get(k) == Some(v))
            })
            .collect())
    }

    fn list_secret_collections(&self) -> io::Result<Vec<String>> {
        Ok(self.collections.keys().cloned().collect())
    }

    fn secret_collection_exists(&self, collection: &str) -> bool {
        self.collections.contains_key(collection)
    }

    fn create_secret_collection(&mut self, collection_name: &str, _label: &str) -> io::Result<()> {
        self.collections
            .entry(collection_name.to_string())
            .or_default();
        Ok(())
    }

    fn delete_secret_collection(&mut self, collection_name: &str) -> io::Result<u32> {
        Ok(self
            .collections
            .remove(collection_name)
            .map(|items| items.len() as u32)
            .unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_routes_put_get_search_delete_without_transport() {
        let mut core = SecretServiceCore::new(MemorySecretStore::new());

        core.dispatch(SecretRequest::Put {
            collection: "default".to_string(),
            key: "api-key".to_string(),
            secret: b"secret".to_vec(),
            label: "API key".to_string(),
            attributes: vec![("service".to_string(), "mail".to_string())],
        })
        .unwrap();

        let response = core
            .dispatch(SecretRequest::Get {
                collection: "default".to_string(),
                key: "api-key".to_string(),
            })
            .unwrap();
        let SecretResponse::Secret(Some(entry)) = response else {
            panic!("expected secret entry");
        };
        assert_eq!(entry.secret, b"secret");
        assert_eq!(entry.meta.label, "API key");

        let response = core
            .dispatch(SecretRequest::Search {
                collection: "default".to_string(),
                attributes: vec![("service".to_string(), "mail".to_string())],
            })
            .unwrap();
        let SecretResponse::Items(items) = response else {
            panic!("expected search results");
        };
        assert_eq!(items.len(), 1);

        let response = core
            .dispatch(SecretRequest::Delete {
                collection: "default".to_string(),
                key: "api-key".to_string(),
            })
            .unwrap();
        assert!(matches!(response, SecretResponse::Deleted(true)));
    }
}
