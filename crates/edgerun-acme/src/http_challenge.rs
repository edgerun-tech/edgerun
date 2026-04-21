use std::collections::HashMap;
use std::sync::Arc;

use sha2::{Sha256, Digest};

use edgerun_http::{Handler, Request, Response, StatusCode};
use edgerun_encoding::base64url_nopad_encode;

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
        format!("{}.{}", token, thumbprint)
    }

    pub fn compute_digest(token: &str, thumbprint: &str) -> String {
        let key_authz = Self::compute_key_authorization(token, thumbprint);
        let mut hasher = Sha256::new();
        hasher.update(key_authz.as_bytes());
        base64url_nopad_encode(&hasher.finalize())
    }
}

impl Handler for HttpChallengeHandler {
    fn handle(&self, _req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let path = _req.uri().path();
            
            let expected_path = format!("/.well-known/acme-challenge/{}", self.token);
            
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
    handlers: Arc<std::sync::RwLock<HashMap<String, HttpChallengeHandler>>>,
    thumbprint: String,
}

impl HttpChallengeServer {
    pub fn new(thumbprint: String) -> Self {
        Self {
            handlers: Arc::new(std::sync::RwLock::new(HashMap::new())),
            thumbprint,
        }
    }

    pub fn add_challenge(&self, token: &str) -> String {
        let key_authz = HttpChallengeHandler::compute_key_authorization(token, &self.thumbprint);
        let handler = HttpChallengeHandler::new(token, &key_authz);
        let path = format!("/.well-known/acme-challenge/{}", token);
        self.handlers.write().unwrap().insert(path.clone(), handler);
        key_authz
    }

    pub fn remove_challenge(&self, token: &str) {
        let path = format!("/.well-known/acme-challenge/{}", token);
        self.handlers.write().unwrap().remove(&path);
    }

    pub fn clear_all(&self) {
        self.handlers.write().unwrap().clear();
    }
}

impl Handler for HttpChallengeServer {
    fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let handlers = Arc::clone(&self.handlers);
        
        Box::pin(async move {
            let path = req.uri().path().to_string();
            
            let handler = handlers.read().unwrap().get(&path).cloned();
            
            if let Some(handler) = handler {
                handler.handle(req).await
            } else {
                Response::not_found()
            }
        })
    }
}