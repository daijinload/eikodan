import { NextResponse } from "next/server";
import { increment } from "../../../lib/db";

// POST increment = DB 書き込み往復。lastshot の POST /increment に対応する
// クリーンな計測点(JSON で増えた後の値を返す)。
export const dynamic = "force-dynamic";

export async function POST() {
  const value = await increment();
  return NextResponse.json({ value });
}
