-- カウンターの初期行。既にあれば何もしない(冪等なので db-setup を何度流してもよい)。
insert into counter (id, value) values (1, 0)
  on conflict (id) do nothing;
