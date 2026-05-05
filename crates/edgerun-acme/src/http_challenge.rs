use crate::prelude::v1::*;
use alloc::collections::BTreeMap as HashMap;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;

use edgerun_http::{Handler, Request, Response, StatusCode};
use edgerun_rt::RwLock;

use crate::challenge_material::{
    http_01_challenge_path, key_authorization, key_authorization_digest,
};

#[derive(Clone)]
pub struct HttpChallengeHandler {
    pub token: String,
    pub key_authorization: String,
}

impl HttpChallengeHandler {
    pub fn new(token: &str, key_authorization: &str) -> Self {
        Self {
            token: token.to_string(),
            key_authorization: key_authorization.to_string(),
        }
    }

    pub fn compute_key_authorization(token: &str, thumbprint: &str) -> String {
        key_authorization(token, thumbprint)
    }

    pub fn compute_digest(token: &str, thumbprint: &str) -> String {
        key_authorization_digest(token, thumbprint)
    }
}

impl Handler for HttpChallengeHandler {
    fn handle(&self, _req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let path = _req.uri().path();

            let expected_path = http_01_challenge_path(&self.token);

            if path == expected_path {
                Response::text(StatusCode::new(200).unwrap(), &self.key_authorization)
            } else {
                Response::not_found()
            }
        })
    }
}

#[derive(Clone)]
pub struct HttpChallengeServer {
    handlers: Arc<RwLock<HashMap<String, HttpChallengeHandler>>>,
    thumbprint: String,
}

impl HttpChallengeServer {
    pub fn new(thumbprint: String) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            thumbprint,
        }
    }

    pub fn add_challenge(&self, token: &str) -> String {
        let key_authz = HttpChallengeHandler::compute_key_authorization(token, &self.thumbprint);
        let handler = HttpChallengeHandler::new(token, &key_authz);
        let path = http_01_challenge_path(token);
        self.handlers.write().insert(path.clone(), handler);
        key_authz
    }

    pub fn remove_challenge(&self, token: &str) {
        let path = http_01_challenge_path(token);
        self.handlers.write().remove(&path);
    }

    pub fn clear_all(&self) {
        self.handlers.write().clear();
    }
}

impl Handler for HttpChallengeServer {
    fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let handlers = Arc::clone(&self.handlers);

        Box::pin(async move {
            let path = req.uri().path().to_string();

            let handler = handlers.read().get(&path).cloned();

            if let Some(handler) = handler {
                handler.handle(req).await
            } else {
                Response::not_found()
            }
        })
    }
}
