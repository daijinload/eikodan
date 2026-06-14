"use client";

// クライアントコンポーネント。SSR で受け取った初期値を表示し、「+1」で
// POST /api/increment を叩いて数字だけ差し替える(lastshot の HTMX フラグメント
// 差し替えに対応する Next.js 流のやり方)。
//
// この 'use client' 境界が hydration を生み、ページは React/Next の chunk を
// 読み込む ── これが「1画面あたりのリクエスト数(fan-out)」として観測される。
import { useState } from "react";

export default function Counter({ initial }: { initial: number }) {
  const [value, setValue] = useState(initial);

  async function inc() {
    const res = await fetch("/api/increment", { method: "POST" });
    const json = (await res.json()) as { value: number };
    setValue(json.value);
  }

  return (
    <>
      <div id="count" className="count">
        {value}
      </div>
      <button className="btn" onClick={inc}>
        +1
      </button>
    </>
  );
}
