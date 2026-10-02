import { getMailHarness, getSwappedHarness, Team, useBoard } from "./entities";
import { KeyProps } from "./panel";

const WrappedRow = ({
  apiKey,
  document,
}: KeyProps) => {
  return apiKey.uid;
  //            ^ d: board/entities.ts:2
};

function Defaulted({ apiKey = fallback }: KeyProps) {
  return apiKey.uid;
  //            ^ d: picker board/entities.ts:2, board/namesakes.ts:2
}

function Shadowed(useBoard: () => Crowd) {
  const { auth } = useBoard();
  return auth.viewer;
  //          ^ d: picker board/entities.ts:19, board/namesakes.ts:7
}

function Looped(team: Team) {
  for (const m of team.members) {
    use(m.permission);
    //    ^ d: picker board/entities.ts:10, board/namesakes.ts:4
  }
  for (const m of team.roster) {
    use(m.permission);
    //    ^ d: picker board/entities.ts:10, board/namesakes.ts:4
  }
}

function Twice(props: { href: string } & { href: string }) {
  return props.href;
  //           ^ d: board/namesakes.ts:6
}

function Either(props: KeyProps | { href: string }) {
  return props.href;
  //           ^ d: board/namesakes.ts:6
}

const mail = getMailHarness(true);
mail.submit("/a");
//   ^ d: picker board/entities.ts:31, board/namesakes.ts:11
const swapped = getSwappedHarness();
swapped.submit("/b");
//      ^ d: picker board/entities.ts:31, board/namesakes.ts:11

