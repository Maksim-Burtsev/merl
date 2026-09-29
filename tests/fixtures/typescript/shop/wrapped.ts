// Names of a destructuring or a parameter list wrapped one per line, as prettier writes them
// (#393): `d` lands on the line of the name.
type Waybill = { sender: string; receiver: string };
type SlipProps = { consignee: string; trackingCode: string };

export function addressOf(waybill: Waybill): string {
  const {
    sender,
    receiver,
  } = waybill;
  return sender + receiver;
  //     ^ d: shop/wrapped.ts:8
  //              ^ d: shop/wrapped.ts:9
  //              status: receiver: local
}

export function legOf(
  origin: string,
  destination: string,
): string {
  return origin + destination;
  //              ^ d: shop/wrapped.ts:19
}

export function Slip({
  consignee,
  trackingCode,
}: SlipProps): string {
  return consignee + trackingCode;
  //                 ^ d: shop/wrapped.ts:27
}
