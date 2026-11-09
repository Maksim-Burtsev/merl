import { Chessman } from "./chessman";
import { Corvid } from "./corvid";

// A case two enums of the project share, written without its enum (#1448).
export function perch(bird: Corvid, piece: Chessman): number {
  switch (bird) {
    case Corvid.Rook:
             // ^ d: rookery/corvid.ts:3
      return 1;
  }
  return piece === Chessman.Rook ? 1 : 0;
                         // ^ d: rookery/chessman.ts:2
}
