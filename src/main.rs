use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, Multipart, OriginalUri, Query},
    http::{request::Parts, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Form, Json, Router,
};
use axum_auth::{AuthBasic, AuthBearer};
use axum_macros::debug_handler;
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;

macro_rules! extract_from_request {
    ($parts:expr, $state:expr, $extractor:ident) => {
        $extractor::from_request_parts($parts, $state)
            .await
            .map_err(|err| err.into_response())?
    };
}

#[async_trait]
impl<S> FromRequestParts<S> for CommonRequestParts
where
    S: Send + Sync,
{
    type Rejection = Response;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let method = extract_from_request!(parts, state, Method);
        let args: Query<HashMap<String, String>> = extract_from_request!(parts, state, Query);
        let headers = extract_from_request!(parts, state, HeaderMap);
        let url = extract_from_request!(parts, state, OriginalUri);
        let origin: ConnectInfo<SocketAddr> = extract_from_request!(parts, state, ConnectInfo);
        Ok(CommonRequestParts::new(
            origin.0,
            url,
            method,
            headers,
            Some(args),
        ))
    }
}

impl CommonRequestParts {
    fn new(
        addr: SocketAddr,
        url: OriginalUri,
        method: Method,
        headers: HeaderMap,
        params: Option<Query<HashMap<String, String>>>,
    ) -> Self {
        let Query(params) = params.unwrap_or_default();
        Self {
            headers: MyHeaderMap(headers.clone()),
            args: params,
            method: method.to_string(),
            url: url.to_string(),
            origin: addr.ip().to_string(),
        }
    }
}

#[derive(Serialize)]
struct GetBasicAuthResponse {
    common_request_parts: CommonRequestParts,
    authenticated: bool,
    user: String,
}

#[derive(Serialize)]
struct GetBearerAuthResponse {
    common_request_parts: CommonRequestParts,
    authenticated: bool,
    token: String,
}

#[derive(Serialize)]
struct PostFormResponse {
    common_request_parts: CommonRequestParts,
    form: HashMap<String, String>,
}
#[derive(Serialize)]
struct PostJsonResponse {
    common_request_parts: CommonRequestParts,
    json: Option<Value>,
    data: String,
}

#[derive(Serialize)]
struct PostFileResponse {
    common_request_parts: CommonRequestParts,
    files: HashMap<String, String>,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/get", get(basic_method_handler))
        .route("/post", post(basic_method_handler))
        .route("/delete", delete(basic_method_handler))
        .route("/put", put(basic_method_handler))
        .route("/patch", patch(basic_method_handler))
        .route("/post/json", post(post_json_handler))
        .route("/post/form", post(form_handler))
        .route("/post/file", post(post_file_handler))
        .route("/basic-auth/user/passwd", get(get_basic_auth_handler))
        .route("/bearer", get(get_bearer_auth_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn basic_method_handler(
    common_request_parts: CommonRequestParts,
) -> Json<CommonRequestParts> {
    Json(common_request_parts)
}

async fn form_handler(
    common_request_parts: CommonRequestParts,
    form: Form<HashMap<String, String>>,
) -> Json<PostFormResponse> {
    let Form(form) = form;
    Json(PostFormResponse {
        common_request_parts,
        form,
    })
}

/// Would be nice to be able to separately echo body and json, but this isn't possible
/// since the request body can only be consumed once
async fn post_json_handler(
    common_request_parts: CommonRequestParts,
    json: Option<Json<Value>>,
) -> Json<PostJsonResponse> {
    let data = json
        .as_ref()
        .map(|Json(val)| val.to_string())
        .unwrap_or_default();
    Json(PostJsonResponse {
        common_request_parts,
        json: json.map(|Json(val)| val),
        data,
    })
}

async fn post_file_handler(
    common_request_parts: CommonRequestParts,
    mut multipart: Multipart,
) -> Json<PostFileResponse> {
    let mut data_map = HashMap::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        data_map.insert(
            name.clone(),
            String::from_utf8(data.clone().to_vec()).unwrap(),
        );
    }
    Json(PostFileResponse {
        common_request_parts,
        files: data_map,
    })
}

async fn get_basic_auth_handler(
    common_request_parts: CommonRequestParts,
    AuthBasic((id, password)): AuthBasic,
) -> Result<Json<GetBasicAuthResponse>, (HeaderMap, StatusCode)> {
    let response: Result<Json<GetBasicAuthResponse>, (HeaderMap, StatusCode)> =
        if let Some(password) = password {
            if password == "passwd" {
                Ok(Json(GetBasicAuthResponse {
                    common_request_parts,
                    user: id,
                    authenticated: true,
                }))
            } else {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "WWW-Authenticate",
                    "Basic realm=\"Fake Realm\"".parse().unwrap(),
                );
                Err((headers, StatusCode::UNAUTHORIZED))
            }
        } else {
            let mut headers = HeaderMap::new();
            headers.insert(
                "WWW-Authenticate",
                "Basic realm=\"Fake Realm\"".parse().unwrap(),
            );
            Err((headers, StatusCode::UNAUTHORIZED))
        };

    response
}

async fn get_bearer_auth_handler(
    common_request_parts: CommonRequestParts,
    AuthBearer(token): AuthBearer,
) -> Json<GetBearerAuthResponse> {
    println!("token is {token}");
    Json(GetBearerAuthResponse {
        common_request_parts,
        token: token,
        authenticated: true,
    })
}

#[derive(Serialize)]
struct CommonRequestParts {
    method: String,
    /// The URL parameters
    args: HashMap<String, String>,
    headers: MyHeaderMap,
    url: String,
    origin: String,
}
/// Simple tuple structs to wrap Axum's `HeaderMap` and `HeaderValue` so we
/// can implement `Serialize` for them, which we need because a set of endpoints
/// echo back the headers from the request
struct MyHeaderMap(HeaderMap);
struct MyHeaderValue(HeaderValue);

impl Serialize for MyHeaderValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_str().unwrap())
    }
}

/// Axum's `HeaderMap` is a multimap because http headers can have duplicate keys,
/// for example, "Set-Cookie" might appear twice in the headers.
impl Serialize for MyHeaderMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let inner = self.0.clone();
        let mut seq = serializer.serialize_map(Some(inner.len()))?;
        let mut prev_header: Option<HeaderName> = None;
        for (k, v) in inner {
            // Axum's `HeaderMap` iter will yield Some(HeaderName) for the first
            // of a header used more than once. Subsequent iterations will yield None,
            // indicating that it's a value for the same key (header name) used in the previous
            // iteration. Why did Axum do it this way? I have no clue. I assume it might
            // the implementation somehow easier.
            let key_to_use = if let Some(ref key) = k {
                key
            } else {
                prev_header.as_ref().unwrap()
            };
            seq.serialize_entry(key_to_use.as_str(), v.to_str().unwrap())?;
            prev_header = k.clone();
        }
        seq.end()
    }
}
