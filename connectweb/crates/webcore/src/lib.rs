//! 共有Web基盤。テンプレートエンジン（MiniJinja + autoreload）とアプリ状態を提供する。
//!
//! 3つのレンダリング口を持つ:
//! - [`AppState::render`]          … 任意の context を渡す素のレンダリング。
//! - [`AppState::render_view`]     … connectweb の肝。**スキーマ生成型インスタンスを1つ**渡すと、
//!   それを minijinja に描画しつつ、**同じインスタンス**を HTMLコメント `<!-- view-data ... -->`
//!   として `</body>` 直前に埋め込む。「この画面が実際に使ったデータ」が、別リクエストとの
//!   ズレなく view-source で読める。コメントなのであくまでデバッグ用の覗き窓で、JS/DOM の
//!   一部にはならない（`<script>` タグや id で本番DOMを汚さない）。
//! - [`AppState::render_view_fragment`] … HTMX部分更新（フラグメント）用。`render_view` と同様に
//!   生成型インスタンスを描画しつつ、同じインスタンスを `<!-- view-data ... -->` コメントで
//!   **先頭**に付ける（`<!doctype>` の無い断片なので先頭でよく、上から読むときデータが先に見える）。

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
    /// `</body>` 直前に `<!-- view-data ... -->` コメントとして埋め込まれる。データ生成は
    /// 1回・出口は2つ（描画と埋め込み）なので、画面の値と埋め込みJSONがズレようがない。
    /// コメントなので本番DOMを汚さず、view-source / DevTools のデバッグ用に読めるだけ。
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
        Html(insert_view_comment(html, &json))
    }

    /// HTMX部分更新（フラグメント）用。`render_view` と同じく生成型インスタンスを1つ渡すと、
    /// それを描画しつつ、**同じインスタンス**を HTMLコメント `<!-- view-data ... -->` として
    /// **先頭**に付ける（レスポンスを上から読むときデータが先に見えるように）。
    ///
    /// `render_view` がフルページの `</body>` 直前に置くのに対し、断片には `<!doctype>` も
    /// `</body>` も無いので先頭に置く。どちらもコメント形式なので本番DOMを汚さず、
    /// view-source / DevTools(Network→Response) で「この断片が使ったデータ」を読むためのもの。
    pub fn render_view_fragment<T: Serialize>(&self, name: &str, view: &T) -> Html<String> {
        let env = match self.reloader.acquire_env() {
            Ok(env) => env,
            Err(e) => return Html(render_error("acquire env", &e.to_string())),
        };
        let tmpl = match env.get_template(name) {
            Ok(t) => t,
            Err(e) => return Html(render_error(&format!("template '{name}'"), &e.to_string())),
        };
        // render_view と同じく、描画と埋め込みで同じ serde 表現を使う（ズレ防止）。
        let value = Value::from_serialize(view);
        let html = match tmpl.render(context! { view => value }) {
            Ok(html) => html,
            Err(e) => return Html(render_error(&format!("render '{name}'"), &format!("{e:#}"))),
        };
        let json = match serde_json::to_string_pretty(view) {
            Ok(j) => j,
            Err(e) => {
                return Html(render_error(
                    &format!("encode view JSON for '{name}'"),
                    &e.to_string(),
                ))
            }
        };
        Html(prepend_view_comment(html, &json))
    }
}

/// JSON を `<!-- view-data ... -->` コメントブロック（末尾改行つき）に組み立てる。
/// コメントを途中で閉じる `-->`（および HTML5 が終端扱いする `--!>`）を作らないよう、
/// 連続ハイフン `--` を `- -` に分離する。これは構文破壊対策で、漏らさない設計は
/// ビュー専用スキーマ側で担保する。デバッグ閲覧用なので、JSON文字列値が `--` を含む
/// 稀なケースだけ見た目が変わる（値そのものは別経路の API で厳密に確認できる）。
fn view_comment_block(json: &str) -> String {
    let safe = json.replace("--", "- -");
    format!("<!-- view-data\n{safe}\n-->\n")
}

/// view-data コメントを `</body>` 直前（無ければ末尾）に挿入する（フルページ用）。
/// `<!doctype>` より前に出すと quirks mode を誘発しうるので、先頭ではなく body 内に置く。
fn insert_view_comment(html: String, json: &str) -> String {
    let block = view_comment_block(json);
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

/// view-data コメントを HTML **先頭**に付ける（フラグメント用）。
/// レスポンスを上から読んだとき、DOM断片より先にデータが目に入るようにしている。
/// 断片には `<!doctype>` が無いので先頭でよい。
fn prepend_view_comment(html: String, json: &str) -> String {
    format!("{}{html}", view_comment_block(json))
}

fn render_error(stage: &str, msg: &str) -> String {
    format!(
        "<pre style=\"color:#b91c1c;white-space:pre-wrap;padding:1rem;border:1px solid #b91c1c;\">\
template error [{stage}]\n{msg}</pre>"
    )
}
