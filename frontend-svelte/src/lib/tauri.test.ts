import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cashStatus, listProducts, recordSale, searchByEan, platformLogin, platformLogout, platformMe, getApiBase, apiFetch, setApiBaseOverride } from "./tauri";

afterEach(() => {
  setApiBaseOverride(null);
  vi.unstubAllGlobals();
});

describe("adaptador local", () => {
  it("consulta produtos na API local sem fallback fictício", async () => {
    const product = { id:1,ean:"1",part_number:"A",description:"Real",price_brl_cents:100,stock_qty:2,updated_at:0,min_stock:1 };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([product]), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(listProducts()).resolves.toEqual([product]);
    expect(fetchMock).toHaveBeenCalledWith("http://127.0.0.1:9001/api/products?limit=200&offset=0", undefined);
  });

  it("propaga indisponibilidade e nunca inventa produto", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("", { status: 503 })));
    await expect(searchByEan("789")).rejects.toThrow("HTTP 503");
  });

  it("envia venda ao serviço local com corpo integral", async () => {
    const sale = { sale_uuid:"x",total_brl_cents:100,payment_method:"cash",items_count:1,outbox_id:1,created_at:1 };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(sale), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(recordSale([{ ean:"1",qty:1,price_brl_cents:100 }], { method:"cash",amount_brl_cents:100 }, "T1", "529.982.247-25")).resolves.toEqual(sale);
    const init = fetchMock.mock.calls[0][1] as RequestInit;
    expect(JSON.parse(String(init.body))).toMatchObject({ terminal_id:"T1", consumer_document:"529.982.247-25", payment:{ method:"cash" } });
  });

  it("recusa função exclusivamente desktop fora do Tauri", async () => {
    await expect(cashStatus()).rejects.toThrow("aplicativo desktop");
  });
});

describe("autenticação plataforma (web)", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
    vi.stubGlobal("window", { __COMMERCECTRL_API_URL__: undefined });
    setApiBaseOverride("https://api.test.com");
  });

  afterEach(() => {
    setApiBaseOverride(null);
    vi.unstubAllGlobals();
  });

  it("platformLogin envia credenciais e retorna sessão", async () => {
    const session = { email: "admin@test.com", role: "superadmin" };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(session), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await platformLogin("admin@test.com", "password123");
    expect(result).toEqual(session);
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.test.com/api/v1/platform/auth/login",
      expect.objectContaining({
        method: "POST",
        credentials: "include",
        body: JSON.stringify({ email: "admin@test.com", password: "password123" }),
      })
    );
  });

  it("platformLogin propaga erro da API", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("Credenciais inválidas", { status: 401 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(platformLogin("a@b.com", "wrong")).rejects.toThrow("Credenciais inválidas");
  });

  it("platformLogout chama endpoint correto", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    await platformLogout();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.test.com/api/v1/platform/auth/logout",
      expect.objectContaining({ method: "POST", credentials: "include" })
    );
  });

  it("platformMe retorna sessão quando autenticado", async () => {
    const session = { email: "admin@test.com", role: "superadmin" };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(session), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await platformMe();
    expect(result).toEqual(session);
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.test.com/api/v1/platform/me",
      expect.objectContaining({ credentials: "include" })
    );
  });

  it("platformMe retorna null em erro", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("", { status: 401 }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await platformMe();
    expect(result).toBeNull();
  });

  it("getApiBase usa override quando definido", () => {
    setApiBaseOverride("https://custom.api");
    expect(getApiBase()).toBe("https://custom.api");
  });

  it("getApiBase usa window.__COMMERCECTRL_API_URL__ como fallback", () => {
    setApiBaseOverride(null);
    vi.stubGlobal("window", { __COMMERCECTRL_API_URL__: "https://window.api" });
    expect(getApiBase()).toBe("https://window.api");
  });

  it("getApiBase usa caminho relativo na web quando nada configurado", () => {
    setApiBaseOverride(null);
    vi.stubGlobal("window", { __COMMERCECTRL_API_URL__: undefined });
    expect(getApiBase()).toBe("");
  });

  it("apiFetch inclui credentials e headers JSON", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ ok: true }), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    await apiFetch("/test", { method: "POST", body: JSON.stringify({ a: 1 }) });
    const [, init] = fetchMock.mock.calls[0];
    expect(init).toMatchObject({
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
    });
  });

  it("apiFetch lança erro com corpo da resposta em falha", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("Erro interno", { status: 500 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiFetch("/fail")).rejects.toThrow("Erro interno");
  });

  it("apiFetch lança erro genérico quando resposta sem corpo", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 503 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiFetch("/fail")).rejects.toThrow("Falha na API (HTTP 503)");
  });
});
