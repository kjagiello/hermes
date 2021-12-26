use as_any::AsAny;
use async_trait::async_trait;
use handlebars::Handlebars;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

mod slack;

#[derive(Debug)]
pub enum FactoryError {
    ServiceNotFound,
    ConfigError(String),
}

impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FactoryError::ServiceNotFound => write!(f, "Service not found"),
            FactoryError::ConfigError(s) => write!(f, "Invalid config: {}", s),
        }
    }
}

#[derive(Debug)]
pub enum CallError {
    // TODO: less ambiguous name
    Fail(String),
    ConfigError(String),
    RenderError(String),
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CallError::Fail(s) => write!(f, "Call failure: {}", s),
            CallError::ConfigError(s) => write!(f, "Invalid config: {}", s),
            CallError::RenderError(s) => write!(f, "Render error: {}", s),
        }
    }
}

pub struct Notification {
    /// A raw notification template
    pub template: Arc<HashMap<String, String>>,
    /// A context to render the template with
    pub context: serde_json::Value,
}

#[derive(Debug)]
pub enum RenderError {
    SubTemplateNotFound,
    RenderError(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderError::SubTemplateNotFound => write!(f, "Sub-template not found"),
            RenderError::RenderError(s) => write!(f, "{}", s),
        }
    }
}

impl Notification {
    /// Renders the notification using Handlebars
    ///
    /// # Arguments
    ///
    /// * `subtemplate` - Name of the sub-template to render
    pub fn render(&self, subtemplate: &str) -> Result<String, RenderError> {
        let template = self
            .template
            .get(subtemplate)
            .ok_or(RenderError::SubTemplateNotFound)?;
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        let rendered = handlebars
            .render_template(template, &self.context)
            .map_err(|e| RenderError::RenderError(format!("Failed to render: {}", e)))?;
        Ok(rendered)
    }
}

#[async_trait]
pub trait ServiceFactory {
    /// Instantiates a notifier service from config
    ///
    /// # Arguments
    ///
    /// * `config` - A service specific configuration
    async fn from_config(config: serde_json::Value) -> Result<Arc<dyn Service>, FactoryError>;
}

pub type ServiceFactoryFn =
    fn(
        serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Arc<dyn Service>, FactoryError>> + Send>>;

/// Defines a target that notifications can be sent to, e.g. Slack, Teams, etc
#[async_trait]
pub trait Service: Sync + Send + AsAny {
    /// Sends a notification with the given context
    ///
    /// # Arguments
    ///
    /// * `notification` - A notification to send
    async fn notify(
        &self,
        config: serde_json::Value,
        notification: Notification,
    ) -> Result<(), CallError>;
}

/// Registry of active service instances
#[async_trait]
pub trait ServiceRegistry: Sync + Send {
    /// Sets up a new service instance
    ///
    /// A single service can be instantiated multiple times with different configurations and given
    /// different aliases. This way you can for example target multiple Slack organizations from a
    /// single workflow.
    ///
    /// # Arguments
    ///
    /// * `alias` - Alias to assign to the service instance. This allows for having multiple
    /// instances of a single service
    /// * `service_name` - Name of the service to instantiate, i.e. slack, teams
    /// * `config` - Service config with a dynamic shape. To be validated by a service factory
    async fn setup(
        &self,
        alias: &str,
        service_name: &str,
        config: serde_json::Value,
    ) -> Result<(), FactoryError>;

    /// Retrieves a service instance
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias of a service instance to retrieve
    fn get(&self, alias: &str) -> Option<Arc<dyn Service>>;
}

pub type ServiceRegistryRef = Arc<dyn ServiceRegistry>;

pub mod registries {
    use super::{slack, FactoryError, Service, ServiceFactory, ServiceFactoryFn, ServiceRegistry};
    use async_trait::async_trait;
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// A registry of all the active service instances
    pub struct DefaultServiceRegistry {
        services: HashMap<String, Arc<ServiceFactoryFn>>,
        instances: Arc<Mutex<HashMap<String, Arc<dyn Service>>>>,
    }

    impl DefaultServiceRegistry {
        pub fn with_default_services() -> Arc<Self> {
            Self::with_services(SERVICES.clone())
        }

        pub fn with_services(services: HashMap<String, Arc<ServiceFactoryFn>>) -> Arc<Self> {
            Arc::from(Self {
                services,
                instances: Arc::new(Mutex::new(HashMap::new())),
            })
        }
    }

    #[async_trait]
    impl ServiceRegistry for DefaultServiceRegistry {
        async fn setup(
            &self,
            alias: &str,
            service_name: &str,
            config: serde_json::Value,
        ) -> Result<(), FactoryError> {
            let factory = self
                .services
                .get(service_name)
                .ok_or(FactoryError::ServiceNotFound)?;
            let service = factory(config).await?;
            let mut instances = self.instances.lock();
            instances.insert(alias.into(), service);
            Ok(())
        }

        fn get(&self, alias: &str) -> Option<Arc<dyn Service>> {
            self.instances.lock().get(alias).cloned()
        }
    }

    lazy_static! {
        /// Holds a registry of all the available services
        static ref SERVICES: HashMap<String, Arc<ServiceFactoryFn>> = {
            let services = [
                ("slack", slack::SlackFactory::from_config),
            ];
            let mut factories: HashMap<_, Arc<ServiceFactoryFn>> = HashMap::new();
            for (name, factory) in services {
                factories.insert(name.into(), Arc::new(factory));
            }
            factories
        };
    }
}
