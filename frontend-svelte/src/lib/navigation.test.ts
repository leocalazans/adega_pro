import { describe, expect, it } from "vitest";
import { routes } from "./navigation";

describe("navegação", () => {
  it("não possui rotas ou nomes duplicados", () => {
    expect(new Set(routes.map((route) => route.href)).size).toBe(routes.length);
    expect(new Set(routes.map((route) => route.label)).size).toBe(routes.length);
  });

  it("mantém todas as telas operacionais no menu", () => {
    expect(routes.map((route) => route.href)).toEqual(expect.arrayContaining([
      "/", "/pos", "/products", "/restock", "/reports", "/expenses",
      "/suppliers", "/employees", "/contacts", "/promotions", "/cash-closing", "/display",
    ]));
  });
});
