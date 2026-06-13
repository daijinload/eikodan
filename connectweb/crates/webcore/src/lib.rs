//! 共有Web基盤。テンプレートエンジン（MiniJinja + autoreload）とアプリ状態を提供する。
//!
//! 2つのレンダリング口を持つ:
//! - [`AppState::render`]      … 任意の context を渡す素のレンダリング（HTMXフラグメント等）。
//! - [`AppState::render_view`] … connectweb の肝。**スキーマ生成型インスタンスを1つ**渡すと、
//!   それを minijinja に描画しつつ、**同じインスタンス**を HTML 末尾に
//!   `<script type="application/json" id="view-data">` として埋め込む。
//!   「この画面が実際に使ったデータ」が、別リクエストとのズレなく view-source で読める。

use std::path::PathBuf;
use std::sync::Arc;

use axum::response::Html;
use minijinja::{context, Environment, Value};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

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

    /// テンプレートをレンダリングしてHTMLレスポンスにする（素の context 版）。
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

    /// スキーマ生成型インスタンスを1つ渡してフルページHTMLにする。
    ///
    /// `view` は `{ view => ... }` としてテンプレートに渡り、**同じ instance** が
    /// HTML 末尾に JSON として埋め込まれる。データ生成は1回・出口は2つ（描画と埋め込み）
    /// なので、画面の値と埋め込みJSONがズレようがない。
    ///
    /// テンプレ側は serde 経由 = proto3 JSON の **camelCase** で参照する
    /// （例: proto の `recent_activities` は `view.recentActivities`）。
    pub fn render_view<T: Serialize>(&self, name: &str, view: &T) -> Html<String> {
        let env = match self.reloader.acquire_env() {
            Ok(env) => env,
            Err(e) => return Html(render_error("acquire env", &e.to_string())),
        };
        let tmpl = match env.get_template(name) {
            Ok(t) => t,
            Err(e) => return Html(render_error(&format!("template '{name}'"), &e.to_string())),
        };
        // 描画と埋め込みで同じ serde 表現を使うため、一度だけ Value 化する。
        let value = Value::from_serialize(view);
        let html = match tmpl.render(context! { view => value }) {
            Ok(html) => html,
            Err(e) => return Html(render_error(&format!("render '{name}'"), &format!("{e:#}"))),
        };
        let json = match serde_json::to_string_pretty(view) {
            Ok(j) => j,
            Err(e) => return Html(render_error(&format!("encode view JSON for '{name}'"), &e.to_string())),
        };
        Html(embed_view_json(html, &json))
    }
}

/// `<script type="application/json">` ブロックを `</body>` 直前（無ければ末尾）に挿入する。
fn embed_view_json(html: String, json: &str) -> String {
    // '<' を < に無害化する。値に `</script>` や `<!--` が混じっても
    // ブロックが早期終了/破壊されない（< は JSON 文字列上では '<' と等価）。
    // これは情報漏洩対策ではなく構文破壊対策（漏らさない設計はビュー専用スキーマ側で担保）。
    let safe = json.replace('<', "\\u003c");
    let block = format!("<script type=\"application/json\" id=\"view-data\">\n{safe}\n</script>\n");
    match html.rfind("</body>") {
        Some(pos) => {
            let mut out = String::with_capacity(html.len() + block.len());
            out.push_str(&html[..pos]);
            out.push_str(&block);
            out.push_str(&html[pos..]);
            out
        }
        None => format!("{html}\n{block}"),
    }
}

fn render_error(stage: &str, msg: &str) -> String {
    format!(
        "<pre style=\"color:#b91c1c;white-space:pre-wrap;padding:1rem;border:1px solid #b91c1c;\">\
template error [{stage}]\n{msg}</pre>"
    )
}
