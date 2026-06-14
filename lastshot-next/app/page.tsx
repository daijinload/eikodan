import { getCount } from "../lib/db";
import Counter from "./counter";

// 毎リクエスト DB を読む(静的化・キャッシュを無効化)。lastshot の GET / と同じく
// 「その時点の DB 値」を SSR する。これが Next.js の GET 経路の比較対象。
export const dynamic = "force-dynamic";

export default async function Page() {
  const value = await getCount();
  return (
    <main className="hero">
      <div>
        <Counter initial={value} />
        <p className="note">値は Postgres に保存。再起動しても残ります。</p>
      </div>
    </main>
  );
}
