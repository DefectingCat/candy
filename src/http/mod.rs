use std::{
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::anyhow;
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use dashmap::DashMap;
use mlua::Lua;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};
use tracing::{debug, info, warn};

use crate::{
    config::SettingHost,
    consts::{ARCH, COMMIT, COMPILER, NAME, OS, VERSION},
    middlewares::{add_headers, add_version, logging_route},
    utils::graceful_shutdown,
};

pub mod error;
// handle static file
pub mod serve;
// handle reverse proxy
pub mod reverse_proxy;
// handle lua script
pub mod lua;
// handle http redirect
pub mod redirect;

/// Host configuration
/// use virtual host port as key
/// use SettingHost as value
/// Use port as parent part
/// Use host.route.location as key
/// Use host.route struct as value
/// {
///     80: {
///         "/doc": <SettingRoute>
///     }
/// }
pub static HOSTS: LazyLock<DashMap<u16, SettingHost>> = LazyLock::new(DashMap::new);

pub struct LuaEngine {
    pub lua: Lua,
    /// Lua 共享字典
    #[allow(dead_code)]
    pub shared_table: Arc<DashMap<String, String>>,
}
impl LuaEngine {
    pub fn new() -> Self {
        let lua = Lua::new();
        let shared_table: DashMap<String, String> = DashMap::new();
        let shared_table = Arc::new(shared_table);

        let module = lua.create_table().expect("create table failed");
        let shared_api = lua.create_table().expect("create shared table failed");

        // 创建共享字典到 lua 中
        let shared_table_get = shared_table.clone();
        shared_api
            .set(
                "set",
                lua.create_function(move |_, (key, value): (String, String)| {
                    shared_table_get.insert(key, value.clone());
                    Ok(())
                })
                .expect("create set function failed"),
            )
            .expect("set failed");
        let shared_table_get = shared_table.clone();
        shared_api
            .set(
                "get",
                lua.create_function(move |_, key: String| {
                    let value = shared_table_get.get(&key);
                    match value {
                        Some(value) => Ok(value.clone()),
                        None => {
                            tracing::error!("shared_api: get key not found: {}", key);
                            Ok(String::new())
                        }
                    }
                })
                .expect("create get function failed"),
            )
            .expect("get failed");
        module
            .set("shared", shared_api)
            .expect("set shared_api failed");

        // 日志函数
        module
            .set(
                "log",
                lua.create_function(move |_, msg: String| {
                    info!("Lua: {}", msg);
                    Ok(())
                })
                .expect("create log function failed"),
            )
            .expect("set log failed");

        module.set("version", VERSION).expect("set version failed");
        module.set("name", NAME).expect("set name failed");
        module.set("os", OS).expect("set os failed");
        module.set("arch", ARCH).expect("set arch failed");
        module
            .set("compiler", COMPILER)
            .expect("set compiler failed");
        module.set("commit", COMMIT).expect("set commit failed");

        // 全局变量 candy
        lua.globals()
            .set("candy", module)
            .expect("set candy table to lua engine failed");

        Self { lua, shared_table }
    }
}
/// lua 脚本执行器
pub static LUA_ENGINE: LazyLock<LuaEngine> = LazyLock::new(LuaEngine::new);

pub async fn make_server(host: SettingHost) -> anyhow::Result<()> {
    let mut router = Router::new();
    let host_to_save = host.clone();
    // find routes in config
    // convert to axum routes
    // register routes
    for host_route in &host.route {
        // http redirect
        if host_route.redirect_to.is_some() {
            // resister with location
            // location = "/doc"
            // route: GET /doc/*
            // resister with file path
            // index = ["index.html", "index.txt"]
            // route: GET /doc/index.html
            // route: GET /doc/index.txt
            // register parent path /doc
            let path_morethan_one = host_route.location.len() > 1;
            let route_path = if path_morethan_one && host_route.location.ends_with('/') {
                // first register path with slash /doc
                router = router.route(&host_route.location, get(redirect::redirect));
                debug!("registed route {}", host_route.location);
                let len = host_route.location.len();
                let path_without_slash = host_route.location.chars().collect::<Vec<_>>()
                    [0..len - 1]
                    .iter()
                    .collect::<String>();
                // then register path without slash /doc/
                router = router.route(&path_without_slash, get(redirect::redirect));
                debug!("registed route {}", path_without_slash);
                host_route.location.clone()
            } else if path_morethan_one {
                // first register path without slash /doc
                router = router.route(&host_route.location, get(redirect::redirect));
                debug!("registed route {}", host_route.location);
                // then register path with slash /doc/
                let path = format!("{}/", host_route.location);
                router = router.route(&path, get(redirect::redirect));
                debug!("registed route {}", path);
                path
            } else {
                // register path /doc/
                router = router.route(&host_route.location, get(serve::serve));
                debug!("registed route {}", host_route.location);
                host_route.location.clone()
            };
            // save route path to map
            {
                host_to_save
                    .route_map
                    .insert(route_path.clone(), host_route.clone());
            }
            let route_path = format!("{route_path}{{*path}}");
            // register wildcard path /doc/*
            router = router.route(route_path.as_ref(), get(serve::serve));
            debug!("registed http redirect route: {}", route_path);
            continue;
        }

        // lua script
        if host_route.lua_script.is_some() {
            // papare lua script
            router = router.route(host_route.location.as_ref(), get(lua::lua));
            let route_path = format!("{}{{*path}}", host_route.location);
            router = router.route(route_path.as_ref(), get(lua::lua));
            // save route path to map
            {
                host_to_save
                    .route_map
                    .insert(host_route.location.clone(), host_route.clone());
            }
            debug!("registed lua script route: {}", route_path);
            continue;
        }

        // reverse proxy
        if host_route.proxy_pass.is_some() {
            router = router.route(host_route.location.as_ref(), get(reverse_proxy::serve));
            // register wildcard path /doc/*
            let route_path = format!("{}{{*path}}", host_route.location);
            router = router.route(route_path.as_ref(), get(reverse_proxy::serve));
            // Set request max body size
            if let Some(max_body_size) = host_route.max_body_size {
                router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
            }
            // save route path to map
            {
                host_to_save
                    .route_map
                    .insert(host_route.location.clone(), host_route.clone());
            }
            debug!("registed reverse proxy route: {}", route_path);
            continue;
        }

        // static file
        if host_route.root.is_none() {
            warn!("root field not found for route: {:?}", host_route.location);
            continue;
        }
        // Set request max body size
        if let Some(max_body_size) = host_route.max_body_size {
            router = router.layer(DefaultBodyLimit::max(max_body_size as usize));
        }
        // resister with location
        // location = "/doc"
        // route: GET /doc/*
        // resister with file path
        // index = ["index.html", "index.txt"]
        // route: GET /doc/index.html
        // route: GET /doc/index.txt
        // register parent path /doc
        let path_morethan_one = host_route.location.len() > 1;
        let route_path = if path_morethan_one && host_route.location.ends_with('/') {
            // first register path with slash /doc
            router = router.route(&host_route.location, get(serve::serve));
            debug!("registed route {}", host_route.location);
            let len = host_route.location.len();
            let path_without_slash = host_route.location.chars().collect::<Vec<_>>()[0..len - 1]
                .iter()
                .collect::<String>();
            // then register path without slash /doc/
            router = router.route(&path_without_slash, get(serve::serve));
            debug!("registed route {}", path_without_slash);
            host_route.location.clone()
        } else if path_morethan_one {
            // first register path without slash /doc
            router = router.route(&host_route.location, get(serve::serve));
            debug!("registed route {}", host_route.location);
            // then register path with slash /doc/
            let path = format!("{}/", host_route.location);
            router = router.route(&path, get(serve::serve));
            debug!("registed route {}", path);
            path
        } else {
            // register path /doc/
            router = router.route(&host_route.location, get(serve::serve));
            debug!("registed route {}", host_route.location);
            host_route.location.clone()
        };
        // save route path to map
        {
            host_to_save
                .route_map
                .insert(route_path.clone(), host_route.clone());
        }
        let route_path = format!("{route_path}{{*path}}");
        // register wildcard path /doc/*
        router = router.route(route_path.as_ref(), get(serve::serve));
        debug!("registed static file route: {}", route_path);
    }

    // save host to map
    HOSTS.insert(host.port, host_to_save);

    router = router.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(add_version))
            .layer(middleware::from_fn(add_headers))
            .layer(TimeoutLayer::new(Duration::from_secs(host.timeout.into())))
            .layer(CompressionLayer::new()),
    );

    router = logging_route(router);

    let addr = format!("{}:{}", host.ip, host.port);
    let addr: SocketAddr = addr.parse()?;

    let handle = Handle::new();
    // Spawn a task to gracefully shutdown server.
    tokio::spawn(graceful_shutdown(handle.clone()));

    // check ssl eanbled or not
    // if ssl enabled
    // then create ssl listener
    // else create tcp listener
    if host.ssl && host.certificate.is_some() && host.certificate_key.is_some() {
        let cert = host
            .certificate
            .as_ref()
            .ok_or(anyhow!("certificate not found"))?;
        let key = host
            .certificate_key
            .as_ref()
            .ok_or(anyhow!("certificate_key not found"))?;
        debug!("certificate {} certificate_key {}", cert, key);

        let rustls_config = RustlsConfig::from_pem_file(cert, key).await?;
        info!("listening on https://{}", addr);
        axum_server::bind_rustls(addr, rustls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    } else {
        info!("listening on http://{}", addr);
        axum_server::bind(addr)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    }

    Ok(())
}
