use std::{
    collections::HashMap,
    error::Error,
    fmt,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{ConnectInfo, Query},
    response::Response,
    response::{IntoResponse, Redirect},
    routing::get,
    Extension, Router,
};

use hmac::Hmac;
use jwt::SignWithKey;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use url::Url;
#[derive(Debug)]
struct Config {
    client: Client,
    key: Hmac<Sha256>,

    client_id: String,
    client_secret: String,
    authorized_users: Vec<String>,
    authorized_orgs: Vec<String>,
    authorized_domain: String,
}

pub fn app(
    client_id: String,
    client_secret: String,
    key: Hmac<Sha256>,
    authorized_users: Vec<String>,
    authorized_orgs: Vec<String>,
    authorized_domain: String,
) -> Router {
    let client = reqwest::Client::builder().build().unwrap();

    let config = Config {
        client,
        key,
        client_id,
        client_secret,
        authorized_users,
        authorized_orgs,
        authorized_domain,
    };
    Router::new()
        .route("/", get(login))
        .route("/callback", get(callback))
        .layer(Extension(Arc::new(config)))
        .layer(Extension(Arc::new(RwLock::new(Callbacks::new()))))
}

async fn login(
    Extension(config): Extension<Arc<Config>>,
    Extension(callbacks): Extension<Arc<RwLock<Callbacks>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<LoginParams>,
) -> Response {
    if let Some(callback) = params.callback {
        let url = Url::parse(&callback).unwrap();
        if url.scheme() == "javascript" {
            return "fuck off".into_response();
        }
        if !url.host_str().unwrap().ends_with(&config.authorized_domain) {
            return "bad domain :(".into_response();
        }

        callbacks
            .write()
            .unwrap()
            .insert(addr.ip().to_string(), callback);
        Redirect::temporary(&format!(
            "https://github.com/login/oauth/authorize?client_id={}",
            config.client_id
        ))
        .into_response()
    } else {
        "callback not set".into_response()
    }
}
async fn callback(
    Extension(config): Extension<Arc<Config>>,
    Extension(callbacks): Extension<Arc<RwLock<Callbacks>>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<GithubOauthCallbackParams>,
) -> Response {
    let token = match get_access_token(&config, params.code).await {
        Ok(token) => token,
        Err(e) => {
            return format!("Error in callback() whilst checking token: {}", e).into_response();
        }
    };

    let resp = api_get("https://api.github.com/user", &token, &config)
        .await
        .unwrap();

    let user: GithubUserResponse = resp.json().await.unwrap();

    if config.authorized_users.contains(&user.id.to_string()) {
        let reader = callbacks.read().unwrap();
        let callback = reader.get(&addr.ip().to_string()).unwrap();
        return sign_jwt(&user, &config, &callback);
    }

    let resp = api_get(&user.organizations_url, &token, &config)
        .await
        .unwrap();

    let orgs: Vec<GithubUserOrg> = resp.json().await.unwrap();

    for org in orgs {
        if config.authorized_orgs.contains(&org.id.to_string()) {
            let reader = callbacks.read().unwrap();
            let callback = reader.get(&addr.ip().to_string()).unwrap();
            return sign_jwt(&user, &config, &callback);
        }
    }

    "hey! you aren't supposed to be here!".into_response()
}
fn sign_jwt(user: &GithubUserResponse, config: &Config, callback: &str) -> Response {
    let jwt = user.sign_with_key(&config.key).unwrap();

    Redirect::temporary(&format!("{}?token={}", callback, jwt)).into_response()
}
async fn api_get(
    url: &str,
    token: &str,
    config: &Config,
) -> Result<reqwest::Response, reqwest::Error> {
    config
        .client
        .get(url)
        .header("User-Agent", "CoolElectronics/oauth-proxy-rs-nginx")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
}

async fn get_access_token(config: &Config, code: Option<String>) -> Result<String, Box<dyn Error>> {
    let code = code.ok_or(AuthError("no code lol".into()))?;

    let resp = config
        .client
        .post("https://github.com/login/oauth/access_token")
        .json(&GithubOauthRequest {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            code,
        })
        .header("Accept", "application/json")
        .send()
        .await?;

    let json: GithubOauthResponse = resp.json().await?;
    if let Some(error) = json.error {
        return Err(Box::new(AuthError(error)));
    }

    Ok(json.access_token.unwrap())
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubOauthResponse {
    error: Option<String>,
    access_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubUserResponse {
    login: String,
    id: u32,
    organizations_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubUserOrg {
    login: String,
    id: u32,
}

#[derive(Debug, Clone)]
struct AuthError(String);
impl Error for AuthError {}
impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

type Callbacks = HashMap<String, String>;

#[derive(Debug, Deserialize)]
struct LoginParams {
    callback: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubOauthCallbackParams {
    code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubOauthRequest {
    client_id: String,
    client_secret: String,
    code: String,
}
