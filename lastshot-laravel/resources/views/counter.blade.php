<!doctype html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>counter — lastshot-laravel</title>
    {{-- lastshot / lastshot-next と「同じ画面」に寄せた最小 CSS（手書き＝CSS ビルド差を交絡にしない）。 --}}
    <style>
      :root {
        --base-200: #e5e6e6;
        --base-content: #1f2937;
        --primary: #491eff;
        --primary-content: #ffffff;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0; min-height: 100vh; padding: 1.5rem;
        background: var(--base-200); color: var(--base-content);
        font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
      }
      .wrap { max-width: 48rem; margin: 0 auto; }
      .hero { display: flex; align-items: center; justify-content: center; min-height: 60vh; text-align: center; }
      .count {
        color: var(--primary);
        font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
        font-size: 4.5rem; font-weight: 700; font-variant-numeric: tabular-nums; line-height: 1.1;
      }
      .btn {
        margin-top: 1.5rem; min-width: 16rem; padding: 0.75rem 1.5rem;
        border: none; border-radius: 0.5rem;
        background: var(--primary); color: var(--primary-content);
        font-size: 1rem; font-weight: 600; cursor: pointer;
      }
      .btn:active { transform: translateY(1px); }
      .note { margin-top: 1rem; font-size: 0.75rem; opacity: 0.5; }
    </style>
  </head>
  <body>
    <div class="wrap">
      <main class="hero">
        <div>
          {{-- #count の中身だけを差し替える（lastshot の HTMX フラグメント差し替えに相当）。 --}}
          <div id="count" class="count">{{ $value }}</div>
          <button class="btn" id="inc">+1</button>
          <p class="note">値は Postgres に保存。再起動しても残ります。</p>
        </div>
      </main>
    </div>
    <script>
      // POST /increment（CSRF 対象外）を叩いて数字だけ差し替える。
      document.getElementById("inc").addEventListener("click", async () => {
        const res = await fetch("/increment", { method: "POST" });
        const json = await res.json();
        document.getElementById("count").textContent = json.value;
      });
    </script>
  </body>
</html>
