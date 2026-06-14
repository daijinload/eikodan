# lastshot-laravel — PHP + Laravel 版カウンター

[lastshot 3スタック比較](../lastshot-bench/)の Laravel 実装。lastshot（Rust）と
**全く同じ DB・同じ画面**の DB保存カウンター（数字 ＋「+1」ボタンだけ）を、Laravel の
素の構成（Blade SSR + web ミドルウェア）で組んで横並び計測する。

## 画面と経路

- `GET /` — `CounterController@index` が `select value from counter where id=1` を読み、
  `resources/views/counter.blade.php` を SSR。
- `POST /increment` — `CounterController@increment` が `update ... +1 ... returning value` で
  +1 して `{value}` を JSON で返す（lastshot の `POST /increment` に対応）。
  ベンチで素の POST として叩けるよう **CSRF 対象外**にしてある（`bootstrap/app.php`）。

ロジックは lastshot（`crates/feature-counter` の `get_count` / `increment`）と同じ SQL を
`DB` ファサードで直接叩く。Eloquent は挟まない（`counter` は lastshot の Flyway が所有する
1 行で `created_at` 等も無い）。

## DB は lastshot と共有

`config/database.php` の `pgsql` 接続を使い、`.env` で:

- `DB_HOST=/tmp`（スラッシュ始まり = libpq が **unix ソケット**を使う）、`DB_DATABASE` は
  worktree スロットに追従（`./run` が prod 時に上書き）、`DB_USERNAME` = OS ユーザー（trust = 空パスワード）、
  `DB_SSLMODE=disable`。
- スキーマ（`counter`）は **lastshot の Flyway が所有**。こちらはマイグレーションしない。

> **セッションは `cookie` ドライバ**にしている（`.env`）。Laravel 既定の `database` セッションは
> 毎リクエスト存在しない `sessions` テーブルを叩いて落ちるし、`file` だと無 cookie 負荷で
> 毎回ファイルを書く = 非代表的なディスク I/O が混じる。`cookie` なら web ミドルウェアの実
> オーバーヘッドは残しつつ、lastshot/next に無い保存系アーティファクトを比較に持ち込まない。

## セットアップと起動

PHP / Composer / nginx はネイティブ（Homebrew）で入れる:

```sh
brew install php composer nginx   # php-fpm・opcache・pdo_pgsql 同梱
./run setup                       # composer install（create-project 済みなら不要）
./run prod                        # nginx + php-fpm（opcache+preload + artisan optimize）。計測はこれ
./run stop                        # 停止（php-fpm / nginx 片付け）
./run serve                       # php artisan serve（開発用・単一ワーカ。計測しない）
```

ポートは worktree スロットで nginx=`3200 + slot`（`dan3`→3203）、php-fpm=`9100 + slot`。
`./run prod` がやること（= 実験メモの「本番相当ビルド」）:

1. `.env` の `DB_DATABASE` を現在のスロットに合わせる。
2. `.run/`（gitignore 済み）に nginx / php-fpm / php.ini を生成。php.ini はシステム ini に
   **opcache + preload**（`deploy/preload.php`）を末尾追記。php-fpm は static（ワーカ = コア数×2）。
3. `php artisan optimize`（config / route / view キャッシュ。`.env` を焼き込む）。
4. php-fpm（背面）→ nginx（前面・`daemon off`）。Ctrl-C / 終了で php-fpm も必ず止める。

```sh
curl http://127.0.0.1:3203/                  # 画面（現在値を SSR）
curl -X POST http://127.0.0.1:3203/increment # => {"value":N}
```

> 起動時に opcache preload の `Can't preload unlinked class ...` 警告が出るが**無害**
> （`opcache_compile_file` はコンパイルのみでクラス link はしない。未解決依存のファイルは
> link 段階でスキップされるだけ）。1 回きりで、リクエスト処理には影響しない。

## 計測上の位置づけ

`php artisan serve` は単一ワーカ・毎リクエスト bootstrap で本番ではない。**必ず `./run prod`
（nginx + php-fpm + opcache + preload + optimize）で測る**。比較の文脈・結果・落とし穴は
[`../lastshot-bench/`](../lastshot-bench/) にまとめてある。

---

このプロジェクトは `composer create-project laravel/laravel` で生成した Laravel 13 スケルトンに、
上記のカウンター（route / controller / view）と本番起動の `run` スクリプト・`deploy/` を足したもの。
Laravel 本体のドキュメントは [laravel.com/docs](https://laravel.com/docs) を参照。
