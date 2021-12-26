use kube::api::Api;
use kube::Client;

pub mod templates {
    use super::*;
    use crate::templates::{Template, TemplateError, TemplateRegistry};
    use async_trait::async_trait;
    use k8s_openapi::api::core::v1::ConfigMap;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// A template registry backed by the Kubernetes ConfigMaps
    ///
    /// As there is no practical reason (AFAICS) for the templates to change during a workflow run,
    /// they are fetched only once and then cached for the lifetime of the process.
    pub struct K8sTemplateRegistry {
        templates: Arc<Mutex<HashMap<String, Arc<Template>>>>,
        client: Client,
    }

    impl K8sTemplateRegistry {
        pub async fn new() -> Result<Arc<Self>, String> {
            Ok(Arc::from(Self {
                templates: Arc::new(Mutex::new(HashMap::new())),
                client: Client::try_default()
                    .await
                    .map_err(|e| format!("Kubernetes client error: {:#?}", e))?,
            }))
        }
    }

    #[async_trait]
    impl TemplateRegistry for K8sTemplateRegistry {
        /// Retrieves a ConfigMap template
        ///
        /// # Arguments
        ///
        /// * `name` - The name of the ConfigMap to retrieve the template from
        async fn get(&self, name: &str) -> Result<Arc<Template>, TemplateError> {
            // Check for the template in the cache
            let cached_template = {
                let registry = self.templates.lock();
                registry.get(name).cloned()
            };
            let (cache, template) = match cached_template {
                Some(t) => (false, t),
                None => {
                    // Template was not found in the cache. Retrieve the ConfigMap
                    let client: Api<ConfigMap> = Api::default_namespaced(self.client.clone());
                    let raw_template = client
                        .get(name)
                        .await
                        .map_err(|e| {
                            TemplateError::GenericError(format!(
                                "Failed to retrieve ConfigMap: {}",
                                e
                            ))
                        })
                        .and_then(|cm| {
                            cm.data.ok_or_else(|| {
                                TemplateError::GenericError("ConfigMap missing data".into())
                            })
                        })?;
                    let template: Arc<Template> = Arc::new(raw_template.into_iter().collect());
                    (true, template)
                }
            };

            if cache {
                // Store it in the cache
                let mut registry = self.templates.lock();
                registry.insert(name.into(), template.clone());
            }

            Ok(template)
        }
    }
}

pub mod secrets {
    use super::*;
    use k8s_openapi::api::core::v1::Secret;
    use std::collections::BTreeMap;

    fn decode(secret: &Secret) -> BTreeMap<String, String> {
        let mut res = BTreeMap::new();
        if let Some(data) = secret.data.clone() {
            for (k, v) in data {
                // Accept only data that cleanly converts to utf-8
                if let Ok(b) = std::str::from_utf8(&v.0) {
                    res.insert(k, b.to_string());
                }
            }
        }
        res
    }

    pub async fn get_secret<T>(name: &str) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        let client = Client::try_default()
            .await
            .map_err(|e| format!("Kubernetes client error: {:#?}", e))?;
        let client: Api<Secret> = Api::default_namespaced(client);
        let secret = client
            .get(name)
            .await
            .map_err(|e| format!("Failed to retrieve Secret: {}", e))
            .as_ref()
            .map(decode)?;
        let value = serde_json::to_value(secret)
            .map_err(|e| format!("Could not parse the secret: {}", e))?;
        let concrete = serde_json::from_value(value)
            .map_err(|e| format!("Could not map the secret: {}", e))?;
        Ok(concrete)
    }
}
