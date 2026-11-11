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

/// Browser sessions have no refresh token; activity slides both the server
/// window and this cookie, bounded by the same hard ceiling.
const BROWSER_COOKIE_MAX_AGE: u64 = 30 * 24 * 3600;

pub(crate) fn router(state: Arc<State>) -> Router {
    let router = Router::new()
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
        .route("/remote/v1/ws", get(socket::upgrade));
    #[cfg(feature = "browser-app")]
    let router = router
        .route("/remote/v1/app", get(browser_app))
        .route("/assets/{*path}", get(browser_asset));
    router
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
    headers.insert("content-security-policy","default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'".parse().expect("static"));
    response
}

async fn info(ExtractState(state): ExtractState<Arc<State>>) -> Json<serde_json::Value> {
    Json(
        json!({"protocolMin":1,"protocolMax":1,"hostId":state.service.host_id(),"name":"TikTools","certificateFingerprint":state.fingerprint,"browserApp":browser_app_enabled(&state)}),
    )
}

pub(crate) fn browser_app_enabled(state: &State) -> bool {
    cfg!(feature = "browser-app")
        && state
            .configuration
            .advertised_origin
            .as_deref()
            .is_some_and(|origin| origin.starts_with("https://"))
}

#[cfg(feature = "browser-app")]
async fn browser_app(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    if !browser_app_enabled(&state) {
        return Err(HttpError::forbidden());
    }
    let session = auth::authenticate(&state, &headers, false).await?;
    if !session.browser {
        return Err(HttpError::forbidden());
    }
    session.authority.snapshot()?;
    let html = crate::browser_assets::INDEX.replacen(
        "<head>",
        "<head><meta name=\"tiktools-transport\" content=\"remote-web\">",
        1,
    );
    Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response())
}

#[cfg(feature = "browser-app")]
async fn browser_asset(
    ExtractState(state): ExtractState<Arc<State>>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Result<Response, HttpError> {
    if !browser_app_enabled(&state) {
        return Err(HttpError::forbidden());
    }
    let session = auth::authenticate(&state, &headers, false).await?;
    if !session.browser {
        return Err(HttpError::forbidden());
    }
    session.authority.snapshot()?;
    let Some((bytes, mime)) = crate::browser_assets::asset(uri.path()) else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok((
        [(header::CONTENT_TYPE, mime)],
        axum::body::Body::from(bytes),
    )
        .into_response())
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
            "{SESSION_COOKIE}={}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age={BROWSER_COOKIE_MAX_AGE}",
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
) -> Result<Response, HttpError> {
    let authentication = auth::authenticate(&state, &headers, false).await?;
    let identity = authentication.authority.snapshot()?;
    if !authentication.browser {
        return Ok(Json(json!({"identity":identity,"csrfToken":null})).into_response());
    }
    let csrf = auth::csrf(&state, &authentication)?;
    // Reissue the cookie on every check so an actively used browser is not
    // logged out by the cookie's own expiry while the session is still alive.
    let cookie = format!(
        "{SESSION_COOKIE}={}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age={BROWSER_COOKIE_MAX_AGE}",
        authentication.token
    );
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(json!({"identity":identity,"csrfToken":csrf})),
    )
        .into_response())
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
    let cookie = format!(
        "{TICKET_COOKIE}={ticket}; Secure; HttpOnly; SameSite=Strict; Path=/remote/v1/ws; Max-Age=30"
    );
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(json!({"expiresInSeconds":30})),
    )
        .into_response())
}
