//! `render_view` / `render_view_fragment` のレンダリング契約（DB 不要）。
//!
//! なぜこれを書くか（docs/testing.md「ユニットテストを書く基準」）:
//! view-data コメントの**挿入位置**と `--` の**エスケープ**は、壊れても画面が
//! 壊れない ── 静かに quirks mode を誘発したり、コメントが途中で閉じて
//! デバッグ用の JSON が本文として表示されたりする。HTTP 越しの仕様テストからは
//! 「コメントがある」までしか見えないので、位置と形はここで釘を打つ。
//!
//! ここは仕様（spec_*）ではなくユニット（unit_*）。共有コアの純粋ロジックであり、
//! 人間がレビューする業務契約ではない。

use std::{
    fs,
    path::{Path, PathBuf},
};

use db::PgPool;
use webcore::AppState;

/// Postgres へ一切接続せずに `AppState` を組み立てる。
///
/// `PgPool::connect_lazy` は「接続を必要になったときだけ張る」（sqlx 0.8.6
/// `pool/mod.rs`: "The pool will establish connections only as needed"）。
/// このテストはレンダリングしか叩かないので、DB が動いていなくても通る。
///
/// ただし各テストが `#[tokio::test]` なのは DB のためではない ── プール構築時点で
/// sqlx がメンテナンス用タスクを spawn する（`pool/inner.rs:529`）ため、ランタイム
/// 文脈が無いと "this functionality requires a Tokio context" で落ちる。
/// レンダリング API 自体は同期。
fn state(dirs: Vec<PathBuf>) -> AppState {
    let pool = PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/nowhere")
        .expect("lazy pool must not require a live database");
    AppState::new(dirs, pool, false)
}

/// テスト専用のテンプレートルートを作る。`CARGO_TARGET_TMPDIR` は cargo が
/// 結合テスト向けに用意する作業ディレクトリなので、tempfile を足さずに済む。
fn template_dir(case: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = fs::remove_dir_all(&dir);
    for (name, body) in files {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().expect("template path has a parent"))
            .expect("create template dir");
        fs::write(&path, body).expect("write template");
    }
    dir
}

/// フルページでは view-data コメントが **`</body>` の直前**に入る。
///
/// 先頭（`<!doctype>` より前）に置くと quirks mode を誘発しうるので、body 内で
/// なければならない。「どこかに含まれる」ではなく位置そのものを assert する。
#[tokio::test]
async fn full_page_puts_view_data_just_before_body_close() {
    let dir = template_dir(
        "full_page",
        &[(
            "page.html",
            "<!doctype html><html><body><p>{{ view.value }}</p></body></html>",
        )],
    );

    let html = state(vec![dir])
        .render_view("page.html", &serde_json::json!({ "value": 3 }))
        .0;

    let comment = html
        .find("<!-- view-data")
        .expect("view-data comment exists");
    let body_close = html.find("</body>").expect("</body> exists");
    let doctype = html.find("<!doctype").expect("<!doctype exists");

    assert!(
        comment > doctype,
        "view-data は <!doctype> より後ろ（quirks mode 回避）"
    );
    assert!(comment < body_close, "view-data は </body> の直前");
    assert!(
        html.contains("<p>3</p>"),
        "描画そのものは通常どおり行われる"
    );
}

/// フラグメントでは view-data コメントが **先頭**に付く。
/// 断片には `<!doctype>` も `</body>` も無いので、上から読んだときデータが先に見える。
#[tokio::test]
async fn fragment_prepends_view_data_at_the_head() {
    let dir = template_dir("fragment", &[("count.html", "{{ view.value }}")]);

    let html = state(vec![dir])
        .render_view_fragment("count.html", &serde_json::json!({ "value": 42 }))
        .0;

    assert!(
        html.starts_with("<!-- view-data"),
        "フラグメントは先頭に view-data が付く: {html:?}"
    );
    assert!(
        !html.contains("</body>"),
        "フルページのシェルを巻き込まない"
    );
    // コメントを除いた可視部分がカウント値そのもの（tests-http がこの前提で値を読む）。
    let visible = html
        .rsplit("-->")
        .next()
        .expect("comment terminator")
        .trim();
    assert_eq!(visible, "42");
}

/// **JSON 中の `--` は `- -` に割る。** 割らないと `-->` / `--!>` を作って
/// コメントが途中で閉じ、残りの JSON が本文として画面に表示されてしまう。
#[tokio::test]
async fn double_hyphen_cannot_close_the_comment_early() {
    let dir = template_dir("hyphen", &[("frag.html", "ok")]);

    let html = state(vec![dir])
        .render_view_fragment("frag.html", &serde_json::json!({ "note": "a--b" }))
        .0;

    assert!(html.contains("a- -b"), "連続ハイフンは分離される: {html:?}");
    assert!(!html.contains("a--b"), "生の -- が残ってはいけない");
    // コメントの終端はただ1つ（= 途中で閉じていない）。
    assert_eq!(html.matches("-->").count(), 1, "コメント終端が増えていない");
}

/// テンプレートが見つからないときは panic せず、赤字のエラー HTML を返す。
/// ハンドラが 500 で落ちるのではなく、壊れた箇所が画面に出るのが設計意図。
#[tokio::test]
async fn missing_template_renders_an_error_instead_of_panicking() {
    let dir = template_dir("missing", &[("exists.html", "ok")]);

    let html = state(vec![dir])
        .render_view("nope.html", &serde_json::json!({}))
        .0;

    assert!(
        html.contains("template error"),
        "エラーが画面に出る: {html:?}"
    );
    assert!(html.contains("nope.html"), "どのテンプレートかが分かる");
}

/// 複数のテンプレートルート（app のシェル + 各 feature の `templates/`）は
/// **渡した順に探索し、最初に見つかったものを使う**。
#[tokio::test]
async fn first_matching_template_root_wins() {
    let first = template_dir("roots_first", &[("shared.html", "FIRST")]);
    let second = template_dir(
        "roots_second",
        &[("shared.html", "SECOND"), ("only.html", "ONLY")],
    );

    let app = state(vec![first, second]);

    let shared = app
        .render_view_fragment("shared.html", &serde_json::json!({}))
        .0;
    assert!(
        shared.contains("FIRST"),
        "先に渡したルートが勝つ: {shared:?}"
    );

    // 後続ルートにしか無いテンプレートもちゃんと見つかる（探索が打ち切られない）。
    let only = app
        .render_view_fragment("only.html", &serde_json::json!({}))
        .0;
    assert!(only.contains("ONLY"), "後続ルートも探索される: {only:?}");
}

/// CSS モードは**実行時グローバル `css_built`** で切り替わる。
/// `cfg!(debug_assertions)` に紐付けると debug↔最終確認の往復で Rust 再ビルドが
/// 走るため、コンパイル時ではなく実行時の値であることを固定する。
#[tokio::test]
async fn css_built_is_a_runtime_global_visible_to_templates() {
    let files = [(
        "base.html",
        "{% if css_built %}BUILT{% else %}CDN{% endif %}",
    )];

    let cdn = AppState::new(
        vec![template_dir("css_cdn", &files)],
        PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/nowhere").expect("lazy pool"),
        false,
    );
    let built = AppState::new(
        vec![template_dir("css_built", &files)],
        PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/nowhere").expect("lazy pool"),
        true,
    );

    assert!(cdn
        .render_view_fragment("base.html", &serde_json::json!({}))
        .0
        .contains("CDN"));
    assert!(built
        .render_view_fragment("base.html", &serde_json::json!({}))
        .0
        .contains("BUILT"));
}
