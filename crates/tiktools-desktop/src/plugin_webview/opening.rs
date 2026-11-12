use super::*;

impl PluginUiWindows {
    /// Opens (or focuses, when already open) the isolated window for one
    /// plugin page. Returns whether a new window was created.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &mut self,
        event_loop: &ActiveEventLoop,
        core: &Arc<AppCore>,
        control: &Arc<ControlApi>,
        runtime: &Handle,
        proxy: &EventLoopProxy<DesktopEvent>,
        plugin_id: &str,
        page_id: &str,
    ) -> Result<bool, String> {
        if !is_valid_ui_id(plugin_id) || !is_valid_ui_id(page_id) {
            return Err("invalid plugin or page id".to_owned());
        }
        let key = (plugin_id.to_owned(), page_id.to_owned());
        if let Some(existing) = self.windows.get(&key) {
            if existing.state == PluginWindowState::Closing {
                // A deferred teardown is still queued: finish it now so the
                // fresh window never shares a map entry with the dying one.
                // The queued close carries the old native id and cannot
                // destroy the replacement window.
                self.windows.remove(&key);
            } else {
                existing.handle().focus_window();
                tracing::debug!(
                    plugin = plugin_id,
                    page = page_id,
                    "plugin window already open, focused"
                );
                return Ok(false);
            }
        }
        if self.windows.len() >= 32 {
            return Err("native plugin window capacity exhausted (32)".to_owned());
        }
        let target = core
            .plugin_ui_target(plugin_id, page_id)
            .map_err(|error| error.to_string())?;
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(format!("TikTools — {}", target.name))
                    .with_inner_size(winit::dpi::LogicalSize::new(960_u32, 720_u32))
                    .with_min_inner_size(winit::dpi::LogicalSize::new(640_u32, 480_u32))
                    .with_resizable(true)
                    .with_visible(true),
            )
            .map_err(|error| format!("could not create plugin window: {error}"))?;

        let assets = PluginAssetServer::new(target.id.clone(), Arc::new(target.asset_root.clone()));
        let navigation_plugin = target.id.clone();
        let ipc_plugin = target.id.clone();
        let ipc_page = target.page_id.clone();
        let ipc_window_id = window.id();
        let ipc_proxy = proxy.clone();
        let ipc_runtime = runtime.clone();
        let ipc_control = control.clone();
        let ipc_slots = Arc::new(tokio::sync::Semaphore::new(128));
        let handler_slots = Arc::clone(&ipc_slots);
        let generation = Arc::new(AtomicU64::new(
            NEXT_PAGE_GENERATION.fetch_add(1, Ordering::AcqRel),
        ));
        let owner = NEXT_WINDOW_OWNER.fetch_add(1, Ordering::AcqRel);
        let page_generation = generation.clone();
        let handler_generation = generation.clone();
        // Post-navigation bridge readiness requires the init script: verify
        // availability before the WebView is built so a missing bridge
        // surfaces as a loud diagnostic instead of a blank page.
        if !init_script_available() {
            tracing::error!(
                plugin = plugin_id,
                page = page_id,
                reason = failure_reasons::INIT_SCRIPT_MISSING,
                "plugin WebView init script unavailable; not opening"
            );
            return Err("plugin WebView initialization script unavailable".to_owned());
        }
        let (ipc_shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
        let mut builder = WebViewBuilder::new()
            .with_on_page_load_handler(move |event, url| {
                if matches!(event, wry::PageLoadEvent::Started) {
                    let generation = NEXT_PAGE_GENERATION.fetch_add(1, Ordering::AcqRel);
                    page_generation.store(generation, Ordering::Release);
                    tracing::info!(url = %url, generation, "plugin WebView navigation Started");
                } else {
                    tracing::info!(url = %url, "plugin WebView navigation Finished; waiting for bridge Ready");
                }
            })
            .with_devtools(cfg!(debug_assertions) || cfg!(feature = "devtools"))
            .with_autoplay(true)
            .with_initialization_script(INIT_SCRIPT)
            .with_navigation_handler(move |url| {
                let allowed = allows_navigation(&navigation_plugin, &url);
                if !allowed {
                    tracing::warn!(url = %url, plugin = %navigation_plugin, "blocked plugin WebView navigation");
                }
                allowed
            })
            .with_new_window_req_handler(move |url, _features| {
                tracing::debug!(url = %url, "blocked plugin WebView new-window request");
                wry::NewWindowResponse::Deny
            })
            .with_ipc_handler(move |request| {
                // The in-page devtools shortcut never reaches the broker
                // allowlist: opening the inspector is window chrome, not a
                // broker call. The check is allocation-free and size-gated.
                if crate::window::ipc::is_open_devtools_fast(request.body()) {
                    let _ = ipc_proxy.send_event(DesktopEvent::Command(
                        DesktopCommand::OpenPluginDevtools {
                            window_id: ipc_window_id,
                        },
                    ));
                    return;
                }
                // Size-gated before queueing (same bound the broker
                // enforces): the payload moves to Tokio for handling and
                // the response returns through `PluginUiRespond`.
                if request.body().len() > tiktools_control_api::MAX_REQUEST_BYTES {
                    tracing::warn!(plugin = %ipc_plugin, "plugin broker request exceeds size limit");
                    return;
                }
                let Ok(permit) = Arc::clone(&handler_slots).try_acquire_owned() else {
                    tracing::warn!(plugin = %ipc_plugin, "plugin broker admission full or closed");
                    return;
                };
                let generation = handler_generation.load(Ordering::Acquire);
                let task_generation = handler_generation.clone();
                let mut cancelled = shutdown_receiver.clone();
                let raw = request.body().clone();
                let broker = PluginUiBroker::new(ipc_plugin.clone(), ipc_control.clone());
                let proxy = ipc_proxy.clone();
                let plugin_id = ipc_plugin.clone();
                let page_id = ipc_page.clone();
                ipc_runtime.spawn(async move {
                    let admission = Arc::new(crate::event::NativeAdmission::new(permit));
                    if *cancelled.borrow_and_update() || task_generation.load(Ordering::Acquire) != generation { return; }
                    let (mut response, effect) = tokio::select! {
                        biased;
                        _ = cancelled.changed() => return,
                        result = tokio::time::timeout(std::time::Duration::from_secs(30), broker.handle(&raw)) => {
                            match result {
                                Ok(result) => result,
                                Err(_) => { tracing::warn!(plugin = %plugin_id, "plugin broker host request timed out"); return; },
                            }
                        },
                    };
                    // Rejections are host-side diagnostics (a page that
                    // only sees `broker error` cannot tell a bad method
                    // from a dead window): log them with owner context.
                    // Responses are small host-generated envelopes.
                    if let Ok(value) = serde_json::from_str::<Value>(&response) {
                        if value.get("ok") == Some(&Value::Bool(false)) {
                            tracing::debug!(
                                plugin = %plugin_id,
                                page = %page_id,
                                error = %value
                                    .get("error")
                                    .and_then(|error| error.as_str())
                                    .unwrap_or("broker error"),
                                "plugin broker rejected native request"
                            );
                        }
                    }
                    if response.len() > crate::window::outbox::MAX_OUTBOUND_BYTES {
                        let id = serde_json::from_str::<Value>(&response).ok().and_then(|v| v.get("id").cloned()).unwrap_or(Value::Null);
                        response = serde_json::json!({"apiVersion": 1, "id": id, "ok": false, "error": "broker response exceeds transport limit"}).to_string();
                    }
                    let _ = proxy.send_event(DesktopEvent::Command(
                        DesktopCommand::PluginUiRespond {
                            window_id: ipc_window_id, plugin_id, page_id,
                            response: response.into(), effect: effect.map(Into::into), generation, admission,
                        },
                    ));
                });
            });
        // Physical bounds from the start (see the main window): some
        // XWayland/KDE sessions report zero screen dimensions, which breaks
        // the logical-default conversion before the WebView attaches.
        let initial_size = window.inner_size();
        builder = builder.with_bounds(Rect {
            position: WryPhysicalPosition::new(0, 0).into(),
            size: WryPhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1))
                .into(),
        });
        builder = builder
            .with_custom_protocol(PLUGIN_ASSET_SCHEME.to_owned(), move |_id, request| {
                assets.respond(request)
            });
        let url = plugin_page_url(&target.id, &target.entry_file, &target.page_id);
        builder = builder.with_url(url.as_str());
        let webview = platform::build_webview(builder, &window)
            .map_err(|error| format!("could not create plugin WebView: {error}"))?;
        window.focus_window();
        let lifecycle = PluginPageSupervisor::new(std::time::Instant::now());
        self.windows.insert(
            key,
            PluginUiWindow {
                webview: Some(webview),
                window: Some(window),
                topics: HashSet::new(),
                state: PluginWindowState::Open,
                ipc_shutdown,
                ipc_slots,
                output: crate::window::outbox::WebviewOutbox::new(),
                generation,
                output_generation: 0,
                owner,
                lifecycle,
            },
        );
        tracing::info!(
            plugin = plugin_id,
            page = page_id,
            transport = "native",
            lifecycle = ?WebviewLifecycle::Loading,
            "plugin window opened: lifecycle Loading, waiting for bridge Ready"
        );
        Ok(true)
    }
}
