import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "counter — lastshot-next",
  description: "DB保存カウンター（Next.js 版・lastshot と同じ Postgres を共有）",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="ja">
      <body>
        <div className="wrap">{children}</div>
      </body>
    </html>
  );
}
