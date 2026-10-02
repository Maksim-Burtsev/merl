import StatusPill from "@/components/StatusPill.vue";

export default function OrdersPage() {
  return (
    <section className="page-list">
      <h1>Orders</h1>
      <p>Pick an order to see its status.</p>
      <StatusPill />
    </section>
  );
}

type Props = { page?: number };
