/**
 * Vercel Cron target. It deliberately calls the cheap backend health endpoint
 * rather than a business route, so it cannot create or change tenant data.
 */
export async function GET(request: Request): Promise<Response> {
  const secret = process.env.CRON_SECRET;
  const authorization = request.headers.get("authorization");

  if (!secret || authorization !== `Bearer ${secret}`) {
    return new Response("Unauthorized", { status: 401 });
  }

  const backendUrl = process.env.COMMERCECTRL_BACKEND_URL;
  if (!backendUrl) {
    return Response.json({ ok: false, error: "COMMERCECTRL_BACKEND_URL is not configured" }, { status: 503 });
  }

  try {
    const response = await fetch(new URL("/health", backendUrl), {
      headers: { "user-agent": "commercectrl-vercel-healthcheck/1.0" },
      signal: AbortSignal.timeout(10_000),
      cache: "no-store",
    });

    return Response.json({ ok: response.ok, backendStatus: response.status }, { status: response.ok ? 200 : 503 });
  } catch {
    return Response.json({ ok: false, error: "backend unavailable" }, { status: 503 });
  }
}
