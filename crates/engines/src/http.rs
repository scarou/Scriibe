//! HTTP derrière un trait : les moteurs ne parlent pas à `ureq` directement, pour que les tests n'aient besoin d'aucun réseau.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Head,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout: Duration,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>, timeout: Duration) -> Self {
        Self { method: Method::Get, url: url.into(), headers: Vec::new(), body: Vec::new(), timeout }
    }

    pub fn post(url: impl Into<String>, body: Vec<u8>, timeout: Duration) -> Self {
        Self { method: Method::Post, url: url.into(), headers: Vec::new(), body, timeout }
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Le serveur n'a pas répondu (réseau coupé, port fermé, délai dépassé). Un code 4xx ou 5xx n'est **pas** une erreur ici.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkError(pub String);

pub trait Http: Send + Sync {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError>;
}

/// Le vrai réseau.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self { agent: ureq::AgentBuilder::new().build() }
    }
}

impl Http for UreqHttp {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        let method = match request.method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Head => "HEAD",
        };
        let mut call = self.agent.request(method, &request.url).timeout(request.timeout);
        for (name, value) in &request.headers {
            call = call.set(name, value);
        }
        let result = if request.body.is_empty() && request.method != Method::Post { call.call() } else { call.send_bytes(&request.body) };
        let response = match result {
            Ok(r) => r,
            // 4xx et 5xx : une réponse comme une autre, que l'appelant interprète.
            Err(ureq::Error::Status(_, r)) => r,
            Err(e) => return Err(NetworkError(e.to_string())),
        };
        let status = response.status();
        let mut body = Vec::new();
        std::io::Read::read_to_end(&mut response.into_reader(), &mut body).map_err(|e| NetworkError(e.to_string()))?;
        Ok(HttpResponse { status, body })
    }
}

/// Une erreur d'un service, avec un message prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceError {
    pub message: String,
    pub status: Option<u16>,
}

impl ServiceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), status: None }
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ServiceError {}

/// Envoie la requête et traduit les échecs en messages clairs. `local` : le service tourne sur cet ordinateur.
pub fn send(http: &dyn Http, request: HttpRequest, service: &str, local: bool) -> Result<Vec<u8>, ServiceError> {
    let response = http.send(request).map_err(|_| {
        ServiceError::new(if !local {
            format!("{service} injoignable (réseau)")
        } else if service == "llama.cpp" {
            format!("llama.cpp ne répond pas ({} le relance à la prochaine dictée)", crate::APP_NAME)
        } else {
            format!("{service} ne répond pas — lance-le sur cet ordinateur")
        })
    })?;
    if (200..300).contains(&response.status) {
        return Ok(response.body);
    }
    let status = Some(response.status);
    let message = match response.status {
        401 | 403 => format!("Clé {service} refusée"),
        402 => format!("Crédit {service} épuisé"),
        429 => format!("{service} : trop de requêtes"),
        other => format!("{service} : erreur {other}"),
    };
    Err(ServiceError { message, status })
}
