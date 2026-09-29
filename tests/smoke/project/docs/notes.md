# Release notes for the support team

## 1.8.0

- ⚠️ A refund past the 30-day window needs a manager's approval: open the order, press Refund and write the reason in the note, the payment team reads it.
- Заказы старше года архивируются, их нельзя вернуть через API: пишите в поддержку платёжного провайдера.
- The refund flow was tested with the payment team 👍🏽 on staging, on the cards and on the bank transfers both, twice.

## 1.7.2

- The order list shows 50 orders a page. Ask ops to change PAGE_SIZE if a customer needs more.
- Search by customer finds orders by the customer's number, not by their name.

## 1.7.1

- A cancelled order keeps its total. The customer sees it struck through.
- Orders paid by bank transfer stay pending until the bank confirms, up to two working days.

## 1.7.0

- Shipping jobs run in the worker: an order is marked shipped within a minute of the label.
- The worker retries a failed sync three times, then the order shows up in the ops channel.
- Orders from the old shop are imported read-only: no refunds, no cancellations.

## 1.6.0

- The order page shows the payment provider's reference: quote it when you write to them.
- Customers can cancel an order themselves while it is pending or paid.

## 1.5.0

- The support page lists the last 20 orders of a customer.
- ✔️ Cancelled orders show the reason the customer gave, under the total.
- Refunds by hand go through scripts/refund_by_hand.py until the refund flow ships.

## 1.4.0

- How to run the API on your machine: [the README](../README.md#orders).
