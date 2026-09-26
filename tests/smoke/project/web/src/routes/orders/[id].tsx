import { useEffect, useState } from "react";
import { ApiClient } from "@/api/client";
import { OrderRepo } from "@/orders/repo";
import { OrderStore } from "@/orders/store";
import type { Order } from "@/orders/types";

const store = new OrderStore(new OrderRepo(new ApiClient("/api")));

export default function OrderPage({ id }: { id: number }) {
  const [order, setOrder] = useState<Order | null>(null);

  useEffect(() => {
    store.load(id).then(setOrder);
  }, [id]);

  if (!order) {
    return <p>Loading…</p>;
  }
  return (
    <section>
      <h1>Order {order.id}</h1>
      <p>
        {order.status}, {order.total}
      </p>
      <button onClick={() => store.cancel(order.id).then(setOrder)}>Cancel</button>
    </section>
  );
}
