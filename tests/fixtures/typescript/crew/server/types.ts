export interface ApiContext<
  ReqT = unknown,
> extends BaseContext {
  input: ReqT;
}

export type TreeNode = {
  id: string;
};
