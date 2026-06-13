//! 共有Web基盤。テンプレートエンジン（MiniJinja + autoreload）とアプリ状態を提供する。
//!
//! 各featureクレートは `webcore::AppState` を `Router<AppState>` の状態として共有し、
//! `state.render("feature名/部品名.html", context!{ ... })` でHTMLを返すだけにする。

use std::path::PathBuf;
use std::sync::Arc;

use axum::response::Html;
use minijinja::{Environment, Value};
use minijinja_autoreload::AutoReloader;

/// 全ハンドラで共有する状態。テンプレートのオートリローダを保持する。
#[derive(Clone)]
pub struct AppState {
    reloader: Arc<AutoReloader>,
}

impl AppState {
    /// 複数のテンプレートルート（appのshell + 各featureの `templates/`）を
    /// 横断検索するローダを構築する。debugビルドではファイル変更を監視し、
    /// 保存のたびに再読み込みする（再コンパイル不要）。
    pub fn new(template_dirs: Vec<PathBuf>) -> Self {
        let reloader = AutoReloader::new(move |notifier| {
            let dirs = template_dirs.clone();
            for dir in &dirs {
                notifier.watch_path(dir, true);
            }
            notifier.set_fast_reload(true);

            let mut env = Environment::new();
            // 複数ルートを順に探す自前ローダ。最初に見つかったファイルを使う。
            env.set_loader(move |name| {
                for dir in &dirs {
                    match std::fs::read_to_string(dir.join(name)) {
                        Ok(source) => return Ok(Some(source)),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(e) => {
                            return Err(minijinja::Error::new(
                                minijinja::ErrorKind::InvalidOperation,
                                format!("failed to read template '{name}': {e}"),
                            ))
                        }
                    }
                }
                Ok(None)
            });
            Ok(env)
        });

        Self {
            reloader: Arc::new(reloader),
        }
    }

    /// テンプレートをレンダリングしてHTMLレスポンスにする。
    /// エラーは握りつぶさず画面に赤字で出す（HTMXなら壊れた箇所だけ即わかる）。
    pub fn render(&self, name: &str, ctx: Value) -> Html<String> {
        let env = match self.reloader.acquire_env() {
            Ok(env) => env,
            Err(e) => return Html(render_error("acquire env", &e.to_string())),
        };
        let tmpl = match env.get_template(name) {
            Ok(t) => t,
            Err(e) => return Html(render_error(&format!("template '{name}'"), &e.to_string())),
        };
        match tmpl.render(ctx) {
            Ok(html) => Html(html),
            Err(e) => Html(render_error(&format!("render '{name}'"), &format!("{e:#}"))),
        }
    }
}

fn render_error(stage: &str, msg: &str) -> String {
    format!(
        "<pre style=\"color:#b91c1c;white-space:pre-wrap;padding:1rem;border:1px solid #b91c1c;\">\
template error [{stage}]\n{msg}</pre>"
    )
}
