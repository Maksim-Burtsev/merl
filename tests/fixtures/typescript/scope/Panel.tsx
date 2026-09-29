// A name the file declares wins over its namesakes elsewhere, above or below the cursor (#337).
type PanelProps = {
  title: string;
};

export function Panel({ title }: PanelProps) {
//                               ^ d: scope/Panel.tsx:2
// status: local
  return <Frame>{title}</Frame>;
  //      ^ d: scope/Panel.tsx:13
}

const Frame = styled.div`
  display: flex;
`;

export function settle(s: Settings): number {
//                        ^ d: picker scope/Panel.tsx:22, scope/Panel.tsx:26
  return s.size;
}

interface Settings {
  size: number;
}

namespace Settings {
  export const size = 1;
}
