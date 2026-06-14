/** @type {import('next').NextConfig} */
const nextConfig = {
  // 本番計測用。余計な開発ヘッダや x-powered-by を落とすくらいで、Next.js の素の
  // 振る舞い（RSC + client hydration + chunk fan-out）はそのまま測る ── これが
  // 「言語 × アーキテクチャ」比較の Next.js 側の正体だから。
  poweredByHeader: false,
};

export default nextConfig;
