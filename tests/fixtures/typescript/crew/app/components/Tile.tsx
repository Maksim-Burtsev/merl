import Caption from "./Caption";
import Wrapped from "./Wrapped";

export function Tile() {
  return <Caption>{Wrapped}</Caption>;
  //      ^ d: crew/shared/Caption.tsx:2
  // status: Caption: via import crew/shared/Caption.tsx
  //               ^ d: crew/app/components/Wrapped.ts:3
}
