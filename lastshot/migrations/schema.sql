-- lastshot のサンプル: DB保存カウンターのスキーマ。
-- 値は 1 行(id=1)だけ持つ。increment は UPDATE ... RETURNING で +1 して返す。
-- proto の CounterView.value は int32 なので、ここも integer(int4) に合わせる。
create table if not exists counter (
  id    integer primary key,
  value integer not null default 0
);
