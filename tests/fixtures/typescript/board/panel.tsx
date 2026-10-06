import { ApiKey, Document, getHarness, useBoard } from "./entities";

export type KeyProps = {
  apiKey: ApiKey;
  document: Document;
};

function KeyRow({ apiKey, document }: KeyProps) {
  const { auth } = useBoard();
  const a = apiKey.uid;
  //               ^ d: board/entities.ts:2
  const b = auth.viewer.nickname;
  //                    ^ d: board/entities.ts:6
  for (const membership of document.memberships) {
    use(membership.permission);
    //             ^ d: board/entities.ts:10
  }
  const { uid, ...leftover } = document;
  use(leftover);
  //  ^ d: board/panel.tsx:18
  return document.insightsEnabled;
  //              ^ d: board/entities.ts:15
}

function KeyLink(props: Omit<KeyProps, "document"> & { href: string }) {
  return props.href;
  //           ^ d: board/panel.tsx:25
}

const harness = getHarness();
harness.submit("/api");
//      ^ d: board/entities.ts:31
