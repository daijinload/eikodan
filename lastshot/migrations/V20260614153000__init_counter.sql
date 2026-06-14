-- counter テーブル（DB保存カウンターの土台）。
-- 値は 1 行(id=1)だけ持つ。increment は UPDATE ... RETURNING で +1 して返す。
-- proto の CounterView.value は int32 なので integer(int4) に合わせる。
--
-- Flyway versioned migration: この版数(タイムスタンプ)は一度だけ適用されるので、
-- 旧 schema.sql の "if not exists" のような冪等ガードは付けない（素の DDL を書く）。
create table counter (
  id    integer primary key,
  value integer not null default 0
);
