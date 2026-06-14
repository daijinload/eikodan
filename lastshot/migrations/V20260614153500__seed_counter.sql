-- カウンターの初期行(id=1, value=0)。
-- versioned migration なので init_counter の後に一度だけ適用される
-- （素のテーブルに 1 回だけ走るので on conflict ガードは不要）。
insert into counter (id, value) values (1, 0);
