import { ApiClient } from "@/api/client";
import type { Order } from "./types";

export class OrderRepo {
  constructor(private readonly client: ApiClient) {}

  find(id: number): Promise<Order> {
    return this.client.get<Order>(`/orders/${id}`);
  }

  cancel(id: number): Promise<Order> {
    return this.client.post<Order>(`/orders/${id}/cancel`, {});
  }
}
