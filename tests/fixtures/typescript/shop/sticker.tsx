function Sticker({ size }: { size: number }) {
  return <b>{size}</b>;
}

export const sticker = <Sticker size={2} />;
//                              ^ d: shop/sticker.tsx:1
export const bold = <b title="x" />;
//                     ^ d: none
//                     status: title: argument label
