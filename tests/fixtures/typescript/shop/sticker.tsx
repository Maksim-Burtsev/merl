function Sticker({ size }: { size: number }) {
  return <b>{size}</b>;
}

export const sticker = <Sticker size={2} />;
//                              ^ d: shop/sticker.tsx:1
export const bold = <b title="x" />;
//                     ^ d: none
//                     status: title: argument label
// An apostrophe in JSX text reads as a string that never ends: the arrow's body is unknown, and
// the name past it is the module's (#531). Inside the body an attribute's `=` does not end it.
const tag = { id: 1 };
export const tags = (all: number[]) => <ul>{all.map(tag => <li key={tag}>it's</li>)}{tag.id}</ul>;
//                                                                                   ^ d: shop/sticker.tsx:12
export const rows = (all: number[]) => <ul>{all.map(tag => <li key={tag} title={tag} />)}</ul>;
//                                                                              ^ d: shop/sticker.tsx:15
