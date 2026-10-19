import * as Y from "yjs";
import type MarkdownIt from "markdown-it";
import { openSession } from "../store";

export class Emitter {
  off(name: string) {
    return name;
  }
  after = 0;
}

export function transaction() {
  return 1;
}

export class ReactNode {}

const step = (pos: any) => {
  for (let d = 0; d < 2; d++) {
    const nodeBefore = pos.at(d);
    return nodeBefore;
  }
};

export class Persistence {
  public readonly doc: Y.Doc;

  constructor(doc: Y.Doc) {
    this.doc = doc;
  }

  destroy(md: MarkdownIt) {
    this.doc.off("destroy");
    //       ^ d: none
    //       status: yjs not installed
    md.core.ruler.after("inline");
    //            ^ d: none
    //            status: markdown-it not installed
    openSession.transaction();
    //          ^ d: picker tea/packaged.ts:12
    return step(React.ReactNode);
    //                ^ d: picker tea/packaged.ts:16
  }
}
