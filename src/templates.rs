use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

pub enum TemplateError {
    /// The template was not found
    NotFound,
    /// The template does not comply with the expected format of the template (a key -> template
    /// mapping)
    InvalidFormat(String),
    /// Any other error that might happen during the retrieval, i.e. failing to retrieve a
    /// ConfigMap from Kubernetes
    GenericError(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            TemplateError::NotFound => write!(f, "Template not found"),
            TemplateError::InvalidFormat(s) => write!(f, "Invalid template format: {}", s),
            TemplateError::GenericError(s) => write!(f, "{}", s),
        }
    }
}

/// A notification template
///
/// A simple template can consist of multiple sub-templates. This allows for the services to have
/// more flexibility in how to handle the notifications. For example, the provided Slack service
/// is expecting a "primary" and a "secondary" template, where the primary one is used to
/// create/update a detailed channel message and the "secondary" to provide a less detailed message
/// in the thread under the channel message.
pub type Template = HashMap<String, String>;

/// Provides a way to fetch templates
#[async_trait]
pub trait TemplateRegistry: Sync + Send {
    /// Retrieves a template
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the template to retrieve
    async fn get(&self, name: &str) -> Result<Arc<Template>, TemplateError>;
}

pub type TemplateRegistryRef = Arc<dyn TemplateRegistry>;
