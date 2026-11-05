export type DeliveryState = "new" | "preparing" | "ready" | "collected";
export type DeliveryItem = { name: string; ean: string; checked: boolean };
export type DeliveryOrder = { id: string; provider: "iFood" | "99Food" | "Zé Delivery"; state: DeliveryState; elapsed: string; pickupCode: string; items: DeliveryItem[] };
