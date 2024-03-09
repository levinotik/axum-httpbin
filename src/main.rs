use std::collections::HashMap;

use axum::{
    extract::{ConnectInfo, Extension, Host, Multipart, OriginalUri, Query, RawForm, Request},
    http::{HeaderMap, HeaderName, HeaderValue, Method},
    routing::{delete, get, head, post, put, patch},
    Form,
    Json,
    Router,
};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

#[derive(Serialize)]
struct CommonRequestComponents {
    method: String,
    /// The URL parameters
    args: HashMap<String, String>,
    headers: MyHeaderMap,
    url: String,
    origin: String,
}

#[derive(Serialize)]
struct FormResponse {
    commonRequestComponents: CommonRequestComponents,
    form: HashMap<String, String>,
}
#[derive(Serialize)]
struct PostResponse {
    commonRequestComponents: CommonRequestComponents,
    json: Option<Value>,
    data: String,
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
        .route("/post/form", post(form_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn basic_method_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    url: OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Option<Query<HashMap<String, String>>>,
) -> Json<CommonRequestComponents> {
    let Query(params) = params.unwrap_or_default();
    let res = CommonRequestComponents {
        headers: MyHeaderMap(headers.clone()),
        args: params,
        method: method.to_string(),
        url: url.to_string(),
        origin: addr.ip().to_string(),
    };

    Json(res)
}

async fn form_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    url: OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Option<Query<HashMap<String, String>>>,
    form: Form<HashMap<String, String>>,
) -> Json<FormResponse> {
    let Query(params) = params.unwrap_or_default();
    let Form(form) = form;
    Json(FormResponse {
        commonRequestComponents: CommonRequestComponents {
            headers: MyHeaderMap(headers.clone()),
            args: params,
            method: method.to_string(),
            url: url.to_string(),
            origin: addr.ip().to_string(),
        },
        form
    })
}

/// Would be nice to be able to separately echo body and json, but this isn't possible
/// since the request body can only be consumed once
async fn post_json_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    url: OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Option<Query<HashMap<String, String>>>,
    json: Option<Json<Value>>,
) -> Json<PostResponse> {
    let Query(params) = params.unwrap_or_default();
    let data = json
        .as_ref()
        .map(|Json(val)| val.to_string())
        .unwrap_or_default();
    Json(PostResponse {
        commonRequestComponents: CommonRequestComponents {
            headers: MyHeaderMap(headers.clone()),
            args: params,
            method: method.to_string(),
            url: url.to_string(),
            origin: addr.ip().to_string(),
        },
        json: json.map(|Json(val)| val),
        data,
    })
}

async fn file_handler(url: OriginalUri, mut multipart: Multipart) {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!(
            "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes",
            data.len()
        );
    }
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
