import { afterEach, describe, expect, it, vi } from "vitest";
import { cashStatus, listProducts, recordSale, searchByEan } from "./tauri";

afterEach(() => vi.unstubAllGlobals());

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
