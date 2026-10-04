// Every construct `f` folds in JavaScript, and the cases that once broke it.
const fs = require("fs");

class Store {  // f: 4-42
	constructor(path) {  // f: 5-7
		this.path = path;  // f: 5-7
	}  // f: 5-7

	read(key) {  // f: 9-20
		if (key in this.cache) {  // f: 10-19
			return this.cache[key];  // f: 9-20
		} else if (key.startsWith("_")) {  // f: 12-19
			return null;  // f: 9-20
		} else {  // f: 9-20
			return fs.readFileSync(  // f: 15-18
				this.path,  // f: 9-20
				"utf8",  // f: 9-20
			);  // f: 9-20
		}  // f: 9-20
	}  // f: 9-20

	query() {  // f: 22-41
		const sql = `  // f: 22-41
select *  // f: 22-41
from t  // f: 22-41
`;  // f: 22-41
// a comment at column 0 inside the body  // f: 22-41
		const rows = [  // f: 28-31
			{ name: "a" },  // f: 22-41
			{ name: "b" },  // f: 22-41
		];  // f: 22-41
		try {  // f: 32-40
			return rows.map(row =>  // f: 33-34
				row.name,  // f: 33-34
			);  // f: 22-41
		} catch (err) {  // f: 36-38
			throw err;  // f: 22-41
		} finally {  // f: 22-41
			this.close();  // f: 22-41
		}  // f: 22-41
	}  // f: 22-41
}  // f: 4-42

function pick(value) {  // f: 44-51
	switch (value) {  // f: 45-50
		case 1:  // f: 46-47
			return "small";  // f: 44-51
		default:  // f: 48-49
			return "other";  // f: 44-51
	}  // f: 44-51
}  // f: 44-51

describe("Store", () => {  // f: 53-59
	it("reads", () => {  // f: 54-58
		assert.throws(() => {  // f: 55-57
			new Store();  // f: 55-57
		}, /path/u);  // f: 55-57
	});  // f: 54-58
});  // f: 53-59
