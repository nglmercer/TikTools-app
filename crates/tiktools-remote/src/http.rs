use crate::{
    auth::{self, HttpError, SESSION_COOKIE, TICKET_COOKIE},
    socket,
    state::State,
};
use axum::{
    extract::{
        rejection::JsonRejection, ConnectInfo, DefaultBodyLimit, Request, State as ExtractState,
    },
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tiktools_collaboration::{ClientKind, Collect, Redeem};

pub(crate) fn router(state: Arc<State>) -> Router {
    Router::new()
        .route("/remote/v1/info", get(info))
        .route(
            "/remote/v1/join",
            get(|| async { Html(include_str!("join.html")) }),
        )
        .route(
            "/remote/v1/join.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("join.js"),
                )
            }),
        )
        .route("/remote/v1/pair/redeem", post(redeem))
        .route("/remote/v1/pair/status", post(collect))
        .route("/remote/v1/session", get(bootstrap))
        .route("/remote/v1/session/refresh", post(refresh))
        .route("/remote/v1/session/logout", post(logout))
        .route("/remote/v1/ws-ticket", post(ticket))
        .route("/remote/v1/ws", get(socket::upgrade))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(Arc::clone(&state), boundary))
        .with_state(state)
}

async fn boundary(
    ExtractState(state): ExtractState<Arc<State>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = if state.stopping.is_cancelled() {
        HttpError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "host_unavailable",
        }
        .into_response()
    } else {
        let headers = request.headers();
        let host = headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok());
        let valid_host = headers.get_all(header::HOST).iter().count() == 1
            && host.is_some_and(|value| state.authorities.contains(&value.to_ascii_lowercase()));
        let proxy_allowed = !state.configuration.tls_proxy
            || request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .is_some_and(|peer| peer.0.ip().is_loopback());
        if !valid_host || !proxy_allowed || request.uri().query().is_some() {
            HttpError::forbidden().into_response()
        } else {
            match request.extensions().get::<ConnectInfo<SocketAddr>>() {
                Some(peer) => match state.rate(&format!("source:{}", peer.0.ip())) {
                    Ok(()) => next.run(request).await,
                    Err(error) => error.into_response(),
                },
                None => HttpError::forbidden().into_response(),
            }
        }
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().expect("static"));
    headers.insert("referrer-policy", "no-referrer".parse().expect("static"));
    headers.insert("x-content-type-options", "nosniff".parse().expect("static"));
    headers.insert("content-security-policy","default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'".parse().expect("static"));
    response
}

async fn info(ExtractState(state): ExtractState<Arc<State>>) -> Json<serde_json::Value> {
    Json(
        json!({"protocolMin":1,"protocolMax":1,"hostId":state.service.host_id(),"name":"TikTools","certificateFingerprint":state.fingerprint}),
    )
}

fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, HttpError> {
    input
        .map(|Json(value)| value)
        .map_err(|_| HttpError::invalid())
}

async fn redeem(
    ExtractState(state): ExtractState<Arc<State>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    input: Result<Json<Redeem>, JsonRejection>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let browser = auth::browser_mode(&state, &headers, true)?;
    let input = body(input)?;
    if (input.client_kind == ClientKind::Browser) != browser {
        return Err(HttpError::forbidden());
    }
    let service = Arc::clone(&state.service);
    let request =
        tokio::task::spawn_blocking(move || service.redeem(input, &peer.ip().to_string()))
            .await
            .map_err(|_| HttpError::internal())??;
    Ok(Json(
        json!({"requestId":request.request_id,"expiresAt":request.expires_at}),
    ))
}

async fn collect(
    ExtractState(state): ExtractState<Arc<State>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    input: Result<Json<Collect>, JsonRejection>,
) -> Result<Response, HttpError> {
    let browser = auth::browser_mode(&state, &headers, true)?;
    let input = body(input)?;
    let service = Arc::clone(&state.service);
    let credentials = tokio::task::spawn_blocking(move || {
        service.collect_for(input, &peer.ip().to_string(), browser)
    })
    .await
    .map_err(|_| HttpError::internal())??;
    if browser {
        let cookie = format!(
            "{SESSION_COOKIE}={}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800",
            credentials.access_token
        );
        Ok((
            [(header::SET_COOKIE, cookie)],
            Json(json!({"approved":true,"expiresAt":credentials.expires_at})),
        )
            .into_response())
    } else {
        Ok(Json(credentials).into_response())
    }
}

async fn bootstrap(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HttpError> {
    let authentication = auth::authenticate(&state, &headers, false).await?;
    let identity = authentication.authority.snapshot()?;
    let csrf = if authentication.browser {
        Some(auth::csrf(&state, &authentication)?)
    } else {
        None
    };
    Ok(Json(json!({"identity":identity,"csrfToken":csrf})))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Refresh {
    refresh_token: String,
}

async fn refresh(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
    input: Result<Json<Refresh>, JsonRejection>,
) -> Result<Response, HttpError> {
    if auth::browser_mode(&state, &headers, true)? {
        return Err(HttpError::forbidden());
    }
    let input = body(input)?;
    let service = Arc::clone(&state.service);
    let credentials = tokio::task::spawn_blocking(move || service.refresh(&input.refresh_token))
        .await
        .map_err(|_| HttpError::internal())??;
    Ok(Json(credentials).into_response())
}

async fn logout(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    let authentication = auth::authenticate(&state, &headers, true).await?;
    auth::check_csrf(&state, &authentication, &headers)?;
    let identity = authentication.authority.snapshot()?;
    let service = Arc::clone(&state.service);
    tokio::task::spawn_blocking(move || {
        service.revoke_session(&identity.session_id, &identity.principal_id)
    })
    .await
    .map_err(|_| HttpError::internal())??;
    if authentication.browser {
        if let Ok(mut browser) = state.browser.lock() {
            browser.remove(&tiktools_collaboration::hash_secret(&authentication.token));
        }
        Ok((
            [(
                header::SET_COOKIE,
                format!("{SESSION_COOKIE}=; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
            )],
            Json(json!({"ok":true})),
        )
            .into_response())
    } else {
        Ok(Json(json!({"ok":true})).into_response())
    }
}

async fn ticket(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    let authentication = auth::authenticate(&state, &headers, true).await?;
    if !authentication.browser {
        return Err(HttpError::forbidden());
    }
    auth::check_csrf(&state, &authentication, &headers)?;
    let ticket = auth::issue_ticket(&state, &authentication)?;
    let cookie=format!("{TICKET_COOKIE}={ticket}; Secure; HttpOnly; SameSite=Strict; Path=/remote/v1/ws; Max-Age=30");
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(json!({"expiresInSeconds":30})),
    )
        .into_response())
}
