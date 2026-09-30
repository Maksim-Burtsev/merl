// `type NodeSpec,` in an import list is one of its names, no alias (#343).
export function schema(): NodeSpec {
//                        ^ d: none
  return {};
}
