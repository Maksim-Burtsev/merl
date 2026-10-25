import type { RuleContext as CoreRuleContext, RuleDefinition } from "@lint/core";

export class SourceCode implements TextSourceCode<{
  LangOptions: object;
}> {
  getFirstToken(node: object): object;
}

export namespace SourceCode {
  interface Cursor {
    skip: number;
  }
}

export namespace Rule {
  interface RuleModule extends RuleDefinition<{
    Code: SourceCode;
  }> {
    create(context: RuleContext): object;
  }

  interface RuleContext extends CoreRuleContext<{
    Code: SourceCode;
  }> {}
}
