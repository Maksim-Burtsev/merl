import type { RuleContext as CoreRuleContext, RuleDefinition } from "@lint/core";

export class SourceCode implements TextSourceCode<{
  LangOptions: object;
}> {
  getFirstToken(node: object): object;
  ast: AST.Program;
  getScope(node: object): Scope.Scope;
  getOuter(node: object): Scope.Variable;
  getLost(node: object): Lost.Scope;
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
    create(context: RuleContext): RuleListener;
  }

  interface RuleContext extends CoreRuleContext<{
    Code: SourceCode;
  }> {}

  interface RuleListener {
    onCodePathStart?(codePath: object): void;
  }
}

export namespace AST {
  interface Token {
    range: number[];
  }

  interface Program {
    tokens: Token[];
  }
}

export namespace Scope {
  interface Scope {
    childScopes: Scope[];
    variables: Variable[];
  }

  interface Variable {
    references: object[];
  }
}
