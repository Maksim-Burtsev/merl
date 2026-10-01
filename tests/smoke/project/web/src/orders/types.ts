export type OrderStatus = "pending" | "paid" | "shipped" | "cancelled" | "refunded";

export interface Order {
  id: number;
  status: OrderStatus;
  total: string;
}

export enum ParcelState {
  Packed = "packed",
  Shipped = "shipped",
}
