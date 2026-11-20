//! Copy for dialogs rendered by the host itself, outside the WebView.
//! The full catalogs live in `src/web/shared/i18n`; this is the small subset
//! the desktop needs before it can render a page. Keep the locale list in
//! sync with `src/web/shared/i18n/supported-locales.ts`.

pub(crate) const SUPPORTED_LOCALES: &[&str] = &["en", "es", "pt", "fr", "zh", "ja"];

struct RestartCopy {
    title: &'static str,
    description: &'static str,
    failed_title: &'static str,
    failed_description: &'static str,
}

const RESTART_COPY: &[(&str, RestartCopy)] = &[
    (
        "en",
        RestartCopy {
            title: "Restart TikTools",
            description: "Restart TikTools now? Installed plugins take effect when the app reopens. Active LIVE sessions and automations will stop.",
            failed_title: "Could not restart TikTools",
            failed_description: "TikTools closed cleanly but could not be reopened. Please open it manually; installed plugins are already active.",
        },
    ),
    (
        "es",
        RestartCopy {
            title: "Reiniciar TikTools",
            description: "¿Reiniciar TikTools ahora? Los plugins instalados se activarán cuando la aplicación vuelva a abrirse. Las sesiones LIVE y las automatizaciones activas se detendrán.",
            failed_title: "No se pudo reiniciar TikTools",
            failed_description: "TikTools se cerró correctamente pero no se pudo volver a abrir. Ábrelo manualmente; los plugins instalados ya están activos.",
        },
    ),
    (
        "pt",
        RestartCopy {
            title: "Reiniciar o TikTools",
            description: "Reiniciar o TikTools agora? Os plugins instalados entram em vigor quando a aplicação for reaberta. As sessões LIVE e as automações ativas serão interrompidas.",
            failed_title: "Não foi possível reiniciar o TikTools",
            failed_description: "O TikTools fechou corretamente, mas não pôde ser reaberto. Abra-o manualmente; os plugins instalados já estão ativos.",
        },
    ),
    (
        "fr",
        RestartCopy {
            title: "Redémarrer TikTools",
            description: "Redémarrer TikTools maintenant ? Les plugins installés prendront effet à la réouverture de l'application. Les sessions LIVE et les automatisations actives seront arrêtées.",
            failed_title: "Impossible de redémarrer TikTools",
            failed_description: "TikTools s'est fermé correctement mais n'a pas pu être réouvert. Veuillez le rouvrir manuellement ; les plugins installés sont déjà actifs.",
        },
    ),
    (
        "zh",
        RestartCopy {
            title: "重启 TikTools",
            description: "现在重启 TikTools？已安装的插件将在应用重新打开后生效。正在进行的直播会话和自动化将会停止。",
            failed_title: "无法重启 TikTools",
            failed_description: "TikTools 已正常关闭，但无法重新打开。请手动打开；已安装的插件已经生效。",
        },
    ),
    (
        "ja",
        RestartCopy {
            title: "TikTools を再起動",
            description: "今すぐ TikTools を再起動しますか？インストール済みのプラグインはアプリの再起動後に有効になります。進行中の LIVE セッションと自動化は停止します。",
            failed_title: "TikTools を再起動できませんでした",
            failed_description: "TikTools は正常に終了しましたが、再起動できませんでした。手動で起動してください。インストール済みのプラグインはすでに有効です。",
        },
    ),
];

fn restart_copy(locale: &str) -> &'static RestartCopy {
    &RESTART_COPY
        .iter()
        .find(|(code, _)| *code == locale)
        .unwrap_or(&RESTART_COPY[0])
        .1
}

pub(crate) fn restart_title(locale: &str) -> &'static str {
    restart_copy(locale).title
}

pub(crate) fn restart_description(locale: &str) -> &'static str {
    restart_copy(locale).description
}

pub(crate) fn restart_failed_title(locale: &str) -> &'static str {
    restart_copy(locale).failed_title
}

pub(crate) fn restart_failed_description(locale: &str) -> &'static str {
    restart_copy(locale).failed_description
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_locale_has_restart_copy() {
        assert_eq!(SUPPORTED_LOCALES.len(), RESTART_COPY.len());
        for (code, copy) in RESTART_COPY {
            assert!(SUPPORTED_LOCALES.contains(code), "{code}");
            assert!(!copy.title.is_empty(), "{code}");
            assert!(!copy.description.is_empty(), "{code}");
            assert!(!copy.failed_title.is_empty(), "{code}");
            assert!(!copy.failed_description.is_empty(), "{code}");
        }
        assert_eq!(restart_title("unknown"), restart_title("en"));
    }
}
