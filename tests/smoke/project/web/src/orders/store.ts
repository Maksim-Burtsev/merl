import { OrderRepo } from "./repo";
import type { Order } from "./types";

export class OrderStore {
  private readonly repo: OrderRepo;
  private cache = new Map<number, Order>();

  constructor(repo: OrderRepo) {
    this.repo = repo;
  }

  async load(id: number): Promise<Order> {
    const order = this.cache.get(id) ?? (await this.repo.find(id));
    this.cache.set(id, order);
    return order;
  }

  async cancel(id: number): Promise<Order> {
    const order = await this.repo.cancel(id);
    this.cache.set(id, order);
    return order;
  }
}
