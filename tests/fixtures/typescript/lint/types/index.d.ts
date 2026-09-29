export interface RuleListener {
	onCodePathEnd?(
		codePath: CodePath,
		node: Node,
	): void;
}

export class SourceCode implements TextSourceCode<{
	LangOptions: LanguageOptions;
}> {
	getTokenAfter: SourceCode.Cursor;
}
