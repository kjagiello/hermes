pub mod filters {
    use super::handlers;
    use crate::services::ServiceRegistryRef;
    use crate::templates::TemplateRegistryRef;
    use warp::Filter;

    pub fn routes(
        service_registry: ServiceRegistryRef,
        template_registry: TemplateRegistryRef,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        template_execute(service_registry, template_registry)
    }

    fn template_execute(
        service_registry: ServiceRegistryRef,
        template_registry: TemplateRegistryRef,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "v1" / "template.execute")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_service_registry(service_registry))
            .and(with_template_registry(template_registry))
            .and_then(handlers::dispatch)
    }

    fn with_service_registry(
        registry: ServiceRegistryRef,
    ) -> impl Filter<Extract = (ServiceRegistryRef,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || registry.clone())
    }

    fn with_template_registry(
        registry: TemplateRegistryRef,
    ) -> impl Filter<Extract = (TemplateRegistryRef,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || registry.clone())
    }
}

mod handlers {
    use super::models;
    use crate::services::{Notification, ServiceRegistryRef};
    use crate::templates::TemplateRegistryRef;
    use std::convert::Infallible;

    type CommandResult = Result<String, String>;

    pub async fn dispatch(
        input: models::Input,
        service_registry: ServiceRegistryRef,
        template_registry: TemplateRegistryRef,
    ) -> Result<impl warp::Reply, Infallible> {
        let result = match input.template.plugin.hermes {
            models::Command::Setup(models::CommandSetup { setup: config }) => {
                setup(config, service_registry).await
            }
            models::Command::Notify(models::CommandNotify { notify: config }) => {
                notify(config, service_registry, template_registry).await
            }
        };
        let response = result
            .map(|m| {
                warp::reply::json(&models::Response {
                    node: models::Node {
                        phase: "Succeeded".into(),
                        message: m,
                    },
                })
            })
            .unwrap_or_else(|m| {
                warp::reply::json(&models::Response {
                    node: models::Node {
                        phase: "Failed".into(),
                        message: m,
                    },
                })
            });
        Ok(response)
    }

    async fn setup(
        config: models::ServiceConfig,
        service_registry: ServiceRegistryRef,
    ) -> CommandResult {
        service_registry
            .setup(&config.alias, &config.service, config.config)
            .await
            .map(|()| "Service setup successful".into())
            .map_err(|e| e.to_string())
    }

    async fn notify(
        config: models::NotificationConfig,
        service_registry: ServiceRegistryRef,
        template_registry: TemplateRegistryRef,
    ) -> CommandResult {
        let template = template_registry
            .get(&config.template)
            .await
            .map_err(|e| format!("Template retrieval failed: {}", e))?;
        let service = service_registry
            .get(&config.target)
            .ok_or_else(|| format!("Service instance \"{}\" not found", config.target))?;
        service
            .notify(
                config.config,
                Notification {
                    template,
                    context: config.context,
                },
            )
            .await
            .map(|_| "Notification sent".into())
            .map_err(|e| e.to_string())
    }
}

mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize)]
    pub struct Response {
        pub node: Node,
    }

    #[derive(Debug, Serialize)]
    pub struct Node {
        pub phase: String,
        pub message: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Input {
        pub template: Template,
    }

    #[derive(Debug, Deserialize)]
    pub struct Template {
        pub plugin: Plugin,
    }

    #[derive(Debug, Deserialize)]
    pub struct Plugin {
        pub hermes: Command,
    }

    #[derive(Debug, Deserialize)]
    pub struct ServiceConfig {
        pub alias: String,
        pub service: String,
        pub config: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    pub struct CommandSetup {
        pub setup: ServiceConfig,
    }

    #[derive(Debug, Deserialize)]
    pub struct NotificationConfig {
        pub target: String,
        pub template: String,
        pub context: serde_json::Value,
        pub config: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    pub struct CommandNotify {
        pub notify: NotificationConfig,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub enum Command {
        Setup(CommandSetup),
        Notify(CommandNotify),
    }
}
