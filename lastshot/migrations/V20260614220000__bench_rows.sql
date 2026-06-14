-- ベンチ用テーブル。Rust(lastshot) vs Node の API 速度比較で「重い DB クエリ」を
-- 作るためだけに使う（lastshot 本体の counter とは無関係）。
--
-- /db/heavy が `where s like '%abc%'` で全行を走査して集約するので、行数ぶんの
-- PG CPU を確実に使う（= DB律速領域を再現する）。行数を増減すれば重さを調整できる。
--
-- Flyway versioned migration: 一度だけ適用されるので素の DDL を書く（冪等ガードなし）。
create table bench_rows (
    id integer primary key,
    n integer not null,
    s text not null
);

-- 決定的にseed（random() を使わず再現可能に）。md5 の16進文字列は 0-9a-f だけなので
-- 'abc' を含む行が一定割合で出る = LIKE が全行を実際にスキャンする。
insert into bench_rows (id, n, s)
select
    g,
    (g * 2654435761) % 1000000,
    md5(g::text)
from generate_series(1, 300000) as g;
