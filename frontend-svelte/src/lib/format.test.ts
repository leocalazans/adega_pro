import { describe, expect, it } from "vitest";
import { brl } from "./format";

describe("brl", () => {
  it("formata centavos sem perda de precisão", () => {
    expect(brl(12345)).toContain("123,45");
    expect(brl(-50)).toContain("0,50");
  });
});
