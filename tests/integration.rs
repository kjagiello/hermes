use argo_hermes::server;
use argo_hermes::services::registries::DefaultServiceRegistry;
use argo_hermes::services::ServiceRegistryRef;
use std::sync::Arc;
use warp::http::Response;
use warp::http::StatusCode;
use warp::test::request;

mod mocks {
    use argo_hermes::services::{
        CallError, FactoryError, Notification, Service, ServiceFactory, ServiceFactoryFn,
    };
    use argo_hermes::templates::{Template, TemplateError, TemplateRegistry};
    use async_trait::async_trait;
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Default)]
    pub struct MockTemplateRegistry;

    #[async_trait]
    impl TemplateRegistry for MockTemplateRegistry {
        async fn get(&self, name: &str) -> Result<Arc<Template>, TemplateError> {
            match name {
                "default" => {
                    let mut subtemplates: HashMap<String, String> = HashMap::new();
                    subtemplates.insert("primary".into(), "Message: {{ message}}".into());
                    Ok(Arc::new(subtemplates))
                }
                _ => Err(TemplateError::NotFound),
            }
        }
    }

    pub struct MockServiceFactory;

    #[async_trait]
    impl ServiceFactory for MockServiceFactory {
        async fn from_config(
            service_config: serde_json::Value,
        ) -> Result<Arc<dyn Service>, FactoryError> {
            Ok(Arc::new(MockService {
                service_config,
                calls: Arc::new(Mutex::new(vec![])),
            }))
        }
    }

    pub struct NotificationCall {
        pub config: serde_json::Value,
        pub notification: Notification,
    }

    pub struct MockService {
        pub service_config: serde_json::Value,
        pub calls: Arc<Mutex<Vec<NotificationCall>>>,
    }

    #[async_trait]
    impl Service for MockService {
        async fn notify(
            &self,
            config: serde_json::Value,
            notification: Notification,
        ) -> Result<(), CallError> {
            // TODO: simulate error
            let mut calls = self.calls.lock();
            calls.push(NotificationCall {
                config,
                notification,
            });
            Ok(())
        }
    }

    lazy_static! {
        /// Holds a registry of all the available services
        pub static ref SERVICES: HashMap<String, Arc<ServiceFactoryFn>> = {
            let services = [
                ("mock", MockServiceFactory::from_config),
            ];
            let mut factories: HashMap<_, Arc<ServiceFactoryFn>> = HashMap::new();
            for (name, factory) in services {
                factories.insert(name.into(), Arc::new(factory));
            }
            factories
        };
    }
}

fn deserialize(res: Response<warp::hyper::body::Bytes>) -> serde_json::Result<serde_json::Value> {
    let (_parts, body) = res.into_parts();
    let body = serde_json::from_slice(&body)?;
    Ok(body)
}

#[tokio::test]
async fn test_setup_success() {
    let service_registry: ServiceRegistryRef =
        DefaultServiceRegistry::with_services(mocks::SERVICES.clone());
    let template_registry = Arc::new(mocks::MockTemplateRegistry::default());
    let api = server::filters::routes(service_registry.clone(), template_registry);

    let service_def = serde_json::json!({
        "alias": "default",
        "service": "mock",
        "config": {
            "token": "topsecret123",
        }
    });
    let res = request()
        .method("POST")
        .path("/api/v1/template.execute")
        .json(&serde_json::json!({
            "template": {
                "plugin": {
                    "hermes": {
                        "setup": service_def
                    }
                }
            }
        }))
        .reply(&api)
        .await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        deserialize(res).unwrap(),
        serde_json::json!({
            "node": {
                "phase": "Succeeded",
                "message": "Service setup successful",
            },
        })
    );

    let service_t = service_registry.get("default").unwrap();
    let service = service_t
        .as_any()
        .downcast_ref::<mocks::MockService>()
        .expect("not found");
    assert_eq!(service.service_config, service_def["config"]);
    assert_eq!(service.calls.lock().len(), 0);
}

#[tokio::test]
async fn test_setup_missing_service() {
    let service_registry: ServiceRegistryRef =
        DefaultServiceRegistry::with_services(mocks::SERVICES.clone());
    let template_registry = Arc::new(mocks::MockTemplateRegistry::default());
    let api = server::filters::routes(service_registry.clone(), template_registry);

    let res = request()
        .method("POST")
        .path("/api/v1/template.execute")
        .json(&serde_json::json!({
            "template": {
                "plugin": {
                    "hermes": {
                        "setup": {
                            "alias": "default",
                            "service": "blabla",
                            "config": {
                                "token": "topsecret123",
                            }
                        }
                    }
                }
            }
        }))
        .reply(&api)
        .await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        deserialize(res).unwrap(),
        serde_json::json!({
            "node": {
                "phase": "Failed",
                "message": "Service not found",
            },
        })
    );
}

#[tokio::test]
async fn test_notify_success() {
    let service_registry: ServiceRegistryRef =
        DefaultServiceRegistry::with_services(mocks::SERVICES.clone());
    let template_registry = Arc::new(mocks::MockTemplateRegistry::default());
    let api = server::filters::routes(service_registry.clone(), template_registry);

    service_registry
        .setup("default", "mock", serde_json::json!({}))
        .await
        .expect("Setup failed");

    let context = serde_json::json!({
        "message": "Hello world",
    });
    let config = serde_json::json!({
        "channel": "sandbox",
    });
    let res = request()
        .method("POST")
        .path("/api/v1/template.execute")
        .json(&serde_json::json!({
            "template": {
                "plugin": {
                    "hermes": {
                        "notify": {
                            "target": "default",
                            "template": "default",
                            "context": context,
                            "config": config,
                        }
                    }
                }
            }
        }))
        .reply(&api)
        .await;

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        deserialize(res).unwrap(),
        serde_json::json!({
            "node": {
                "phase": "Succeeded",
                "message": "Notification sent",
            },
        })
    );

    let service_t = service_registry.get("default").unwrap();
    let service = service_t
        .as_any()
        .downcast_ref::<mocks::MockService>()
        .expect("not found");
    let calls = service.calls.lock();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.config, config);
    let rendered = call.notification.render("primary").unwrap();
    assert_eq!(rendered, "Message: Hello world");
}
