//! 生成型 `CounterView` の **proto3 JSON 表現**を固定するユニットテスト。
//!
//! なぜこれを書くか（docs/testing.md「ユニットテストを書く基準」）:
//! この serde 表現は 3 経路が共有する ── テンプレート（minijinja に渡る形）・
//! HTML 末尾の `<!-- view-data -->`・Connect API のレスポンス。**表現が変わると
//! テンプレート側は例外を出さず静かに `undefined` になる**（CLAUDE.md「スキーマ
//! ファーストの掟」）。落ちないバグなので、契約を明示的に釘打ちしておく価値がある。
//!
//! ここは仕様（spec_*）ではなくユニット（unit_*）。ライブラリ（buffa の json feature）の
//! 振る舞いを固定しているだけで、人間がレビューする業務契約ではない。

use schema::CounterView;

/// **proto3 JSON は 0 値フィールドを省略する。** 初期状態のカウンターを埋め込むと
/// `<!-- view-data -->` の中身は `{"value":0}` ではなく `{}` になる。
///
/// これを知らずに「埋め込み JSON に value キーがあること」を assert すると、
/// カウンターが 0 のときだけ落ちるテストができあがる。`tests-http` 側が
/// キー名ではなくコメントの存在で確認しているのは、この性質のため。
#[test]
fn zero_is_omitted_from_proto3_json() {
    let json = serde_json::to_string(&CounterView {
        value: 0,
        ..Default::default()
    })
    .expect("CounterView should serialize");

    assert_eq!(json, "{}", "proto3 JSON は 0 値を省略する");
}

/// 0 以外はそのまま載る。**int32 は素の数値**で出る（int64 にすると proto3 JSON は
/// 文字列 `"7"` にするので、テンプレートの数値比較が壊れる ── counter.proto が
/// int32 を選んでいる理由）。
#[test]
fn non_zero_is_a_bare_number() {
    let json = serde_json::to_string(&CounterView {
        value: 7,
        ..Default::default()
    })
    .expect("CounterView should serialize");

    assert_eq!(json, r#"{"value":7}"#);
    assert!(
        !json.contains(r#""7""#),
        "int32 は文字列化されない（int64 との違い）"
    );
}

/// 省略された 0 値は、読み戻すと 0 になる（省略 = 未設定 = 型の既定値）。
/// 埋め込み JSON を外部ツールが読み戻すときの往復が壊れないことの確認。
#[test]
fn omitted_field_round_trips_to_zero() {
    let view: CounterView = serde_json::from_str("{}").expect("empty object should deserialize");

    assert_eq!(view.value, 0);
}

// NOTE: CLAUDE.md が警告するもう1つの契約 ──「複合語フィールドは camelCase
// （`recent_activities` → `recentActivities`）」── は、現在の counter.proto に
// 複合語フィールドが1つも無いため**まだ検証できない**。複合語フィールドを追加した
// 時点でここにケースを足すこと（追加せずに済ませると、テンプレート側が静かに
// undefined になる例の事故が素通りする）。
